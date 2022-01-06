use crate::config::Config;
use chrono::Utc;
use std::collections::HashMap;
use std::env;
use std::rc::Rc;
// job_scheduler crate https://docs.rs/job_scheduler/1.2.1/job_scheduler/
use crate::job::Job;
use anyhow;
use closure::closure;
use futures::executor::ThreadPool;
use job_scheduler::Schedule;
use std::sync::{Arc, RwLock, RwLockWriteGuard};
use std::thread;
use std::time::Duration;
use tokio::runtime;
use tokio::task;

mod config;
mod db;
mod heartbeat;
mod job;
mod storage;

// Maybe should use an Arc on the Scheduler itself
pub struct Scheduler<'a> {
    // Holds the main Job struct
    jobs: HashMap<String, job::Job>,
    // Used to unschedule a job if needed
    job_ids: HashMap<String, job_scheduler::Uuid>,
    // Used to request to the database only jobs created after it
    last_updated_at: i64,
    job_scheduler: job_scheduler::JobScheduler<'a>,
    config: Config,
}
#[derive(Clone, PartialEq, Eq)]
enum Role {
    LEADER,
    FOLLOWER,
}

impl Scheduler<'_> {
    pub fn new(config: Config) -> Self {
        Self {
            jobs: HashMap::new(),
            job_ids: HashMap::new(),
            last_updated_at: Utc::now().timestamp(),
            job_scheduler: job_scheduler::JobScheduler::new(),
            config,
        }
    }
    pub fn add_jobs(self: &mut Self, jobs: Vec<Job>) -> () {
        for job in &jobs {
            self.jobs.insert(job.name.clone(), job.clone());
        }
    }
}

// Get all the jobs in the database and updates it every 5 min
// Schedule the jobs using job_scheduler and keeps their uuid
// when updating the jobs, we need to hold a write lock
// the job thread should request read lock, and send the job to a worker

#[tokio::main]
pub async fn main() -> () {
    // Gets all the jobs from the database and set jobs
    // Creates the new object
    // and finally runs the Scheduler

    let config_file = match env::var("DCRON_CONFIG") {
        Ok(config_file) => config_file,
        _ => "app.toml".into(),
    };

    let config = Config::from(&config_file);

    let config = match config {
        Ok(config) => config,
        _ => panic!("Error while trying to read configuration file"),
    };

    let role: Arc<RwLock<Role>> = Arc::new(RwLock::new(Role::FOLLOWER));

    let instance_role = role.clone();
    let health_check_config = config.clone();
    tokio::spawn(async move {
        run_health_checks(instance_role, health_check_config);
    });

    // run an infinity loop
    // if this instance is a leader it will create a scheduler
    // and fetch updates for itself
    // otherwise it will be waiting it to become a leader
    run_leader_scheduler(config.clone(), role).await;
}

fn server_name() -> String {
    //TODO get the IP:PORT
    "test".into()
}

fn run_health_checks(health_checks_role: Arc<RwLock<Role>>, config: Config) {
    let pool = ThreadPool::new().unwrap(); // probably move away from here

    loop {
        thread::sleep(Duration::from_millis(5000));
        pool.spawn_ok(heartbeat(config.clone()));
        if let Ok(mut role) = health_checks_role.write() {
            *role = role_should_assume(config.clone()).unwrap();
        }
    }
}

async fn heartbeat(config: Config) {
    if let Ok(db) = db::get_db(&config).await {
        db.send_heartbeat(&server_name());
    } else {
        println!("Could not send heartbeat");
    }
}

fn role_should_assume(config: Config) -> Result<Role, anyhow::Error> {
    // select ips that sent a heartbeat in the last 30 seconds
    // if the smallest IP is us, return Leader
    // otherwise follower
    let basic_rt = runtime::Builder::new_current_thread().build()?;
    let config = config.clone();
    let role = basic_rt.block_on(async {
        let db = match db::get_db(&config).await {
            Ok(db) => db,
            _ => return Role::FOLLOWER,
        };

        let hb = match db.most_recent_heartbeat().await {
            Ok(Some(hb)) => hb,
            _ => return Role::FOLLOWER,
        };

        if hb.server == server_name() {
            Role::LEADER
        } else {
            Role::FOLLOWER
        }
    });
    Ok(role)
}

async fn run_leader_scheduler(config: Config, role: Arc<RwLock<Role>>) -> ! {
    let db = match db::get_db(&config).await {
        Ok(db) => db,
        _ => panic!("Could not get DB"),
    };

    loop {
        if let Ok(role) = role.read() {
            match *role {
                Role::LEADER => println!("Still leader"),
                //TODO we probably can sleep after finding out we are a follower
                Role::FOLLOWER => continue, // Nothing to do, wait until we are the leader
            };
        } else {
            // sleep and retry later
            thread::sleep(Duration::from_millis(50));
            continue;
        }

        let jobs = match db.find_all_active().await {
            Ok(jobs) => jobs,
            _ => panic!("Could not initialise jobs"),
        };

        let mut scheduler = Scheduler::new(config.clone());

        scheduler.add_jobs(jobs);

        let arc_scheduler = Arc::new(RwLock::new(scheduler));

        schedule_all(arc_scheduler.clone()).unwrap();
        // This runs in a loop and only breaks if this instance is not
        // a leader anymore
        fetch_job_updates(arc_scheduler.clone(), role.clone()).await;
    }
}

fn schedule_all(scheduler: Arc<RwLock<Scheduler>>) -> Result<(), anyhow::Error> {
    // Get write lock
    // Schedule all the jobs and setup jobs_id
    // meant to be run once when we start the scheduler
    if let Ok(mut scheduler) = scheduler.write() {
        let jobs = (*scheduler).jobs.clone();
        for (_job_name, job) in jobs {
            schedule_job(job.clone(), &mut *scheduler)?;
        }
    } else {
        return Err(anyhow::anyhow!("Could not get write lock"));
    }
    return Ok(());
}

fn schedule_job(job: job::Job, scheduler: &mut Scheduler) -> Result<(), anyhow::Error> {
    let job_name = job.name.clone();
    let job_id = scheduler.job_scheduler.add(job_scheduler::Job::new(
        (&job.time).parse().unwrap(),
        closure!(move job, || {
            run_job(&job);
        }),
    ));

    scheduler.job_ids.insert(job_name, job_id);
    Ok(())
}

pub fn tick(scheduler: Arc<RwLock<Scheduler>>) -> () {
    if let Ok(mut scheduler) = scheduler.write() {
        (*scheduler).job_scheduler.tick();
        thread::sleep(Duration::from_millis(50));
    } else {
        thread::sleep(Duration::from_millis(5));
    }
}

async fn fetch_job_updates<'a>(
    scheduler: Arc<RwLock<Scheduler<'a>>>,
    role: Arc<RwLock<Role>>,
) -> () {
    // suppose to be run in a new thread
    // main method
    // loop
    // every x minutes, goes to the database
    // Acquires writer lock and unschedule any job that is needed and deletes from job_ids
    // Acquires writer lock and updates jobs.
    // schedule any new job
    loop {
        tick(scheduler.clone()); //TODO: tick has some sleep logic inside and should be run in another thread in loop
        thread::sleep(Duration::from_millis(4000));
        if let Ok(mut scheduler) = scheduler.write() {
            let last_updated_at = (*scheduler).last_updated_at;
            let disabled_jobs = match update_scheduler(&mut *scheduler).await {
                Ok(disabled_jobs) => disabled_jobs,
                _ => continue,
            };
            disable_jobs(&mut scheduler, disabled_jobs);

            reschedule_jobs_if_needed(&mut scheduler, last_updated_at);
        }

        //This is terrible, but for now we also check here if we are still the leader
        // if not we should break and stop updating our scheduler
        if let Ok(role) = role.read() {
            match *role {
                Role::LEADER => println!("Still leader"),
                Role::FOLLOWER => break,
            };
        } // If we don't get the lock, we may act as a leader for longer than we should, but is ok for now
    }
}

fn reschedule_jobs_if_needed(scheduler: &mut RwLockWriteGuard<Scheduler>, last_updated_at: i64) {
    let jobs = (*scheduler).jobs.clone();
    for (_job_name, job) in jobs {
        if job.updated_at >= last_updated_at {
            let uuid = scheduler.job_ids.remove(&job.name);
            match uuid {
                Some(uuid) => scheduler.job_scheduler.remove(uuid),
                None => false,
            };

            schedule_job(job, &mut *scheduler); //TODO should check result
        }
    }
}

fn disable_jobs(scheduler: &mut Scheduler, disabled_jobs: Vec<Job>) {
    for job in disabled_jobs {
        let uuid = scheduler.job_ids.remove(&job.name);
        match uuid {
            Some(uuid) => scheduler.job_scheduler.remove(uuid),
            None => continue,
        };
    }
}

// Update the scheduler and return all jobs that were disabled
// this won't reschedule any job. It's just updating the datastructure
async fn update_scheduler<'a>(
    scheduler: &mut Scheduler<'a>,
) -> Result<Vec<job::Job>, anyhow::Error> {
    let config = &scheduler.config;
    let last_updated_at = scheduler.last_updated_at;
    let db = match db::get_db(config).await {
        Ok(db) => db,
        _ => panic!("Could not get DB"),
    };

    let active_jobs = db.find_all_since(true, last_updated_at).await?;
    scheduler.jobs.clear();

    for job in active_jobs {
        scheduler.jobs.insert(job.name.clone(), job);
    }

    scheduler.last_updated_at = Utc::now().timestamp();
    let deleted_jobs = db.find_all_since(false, last_updated_at).await?;

    Ok(deleted_jobs)
}

fn run_job(job: &job::Job) -> Result<(), anyhow::Error> {
    // Acquires reader lock on job ids (not sure if we really need)
    // If standalone runs the script
    // Otherwise gets a worker IP and sends an execution request to it
    println!("Would execute: {:?}", job);
    Ok(())
}
