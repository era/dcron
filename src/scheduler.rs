use crate::config::Config;
use chrono::Utc;
use std::collections::HashMap;
use std::env;
use std::rc::Rc;
// job_scheduler crate https://docs.rs/job_scheduler/1.2.1/job_scheduler/
use anyhow;
use closure::closure;
use job_scheduler::Schedule;
use std::sync::{Arc, RwLock, RwLockWriteGuard};
use std::thread;
use std::time::Duration;
use tokio::task;
use crate::job::Job;

mod config;
mod db;
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

enum Role {
    LEADER,
    FOLLOWER,
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


    tokio::spawn(async move {
        run_health_checks(instance_role);
    });

    //if leader
    run_leader_scheduler(config, role).await; // run an infinity loop
    // else don't do anything, just keep waiting and checking
    // if we won the electin
}

fn run_health_checks(health_checks_role: Arc<RwLock<Role>>) {
    loop {
        thread::sleep(Duration::from_millis(500));
        //TODO implement the logic here
        if let Ok(mut role) = health_checks_role.write() {
            //TODO for now we only have one instance which is always the leader
            *role = Role::LEADER;
        }
    }
}


async fn run_leader_scheduler(config: Config, role: Arc<RwLock<Role>>) -> !{
    let db = match db::get_db(&config).await {
        Ok(db) => db,
        _ => panic!("Could not get DB"),
    };

    let jobs = match db.find_all_active().await {
        Ok(jobs) => jobs,
        _ => panic!("Could not initialise jobs"),
    };
    let role = &role.clone();
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

        let mut scheduler = Scheduler {
            jobs: HashMap::new(),
            job_ids: HashMap::new(),
            last_updated_at: Utc::now().timestamp(),
            job_scheduler: job_scheduler::JobScheduler::new(),
            config: config.clone(),
        };

        for job in &jobs {
            scheduler.jobs.insert(job.name.clone(), job.clone());
        }

        let arc_scheduler = Arc::new(RwLock::new(scheduler));

        schedule_all(arc_scheduler.clone()).unwrap();

        fetch_job_updates(arc_scheduler.clone(), role.clone()).await; // This runs in a loop

    }
}

fn schedule_all(scheduler: Arc<RwLock<Scheduler>>) -> Result<(), anyhow::Error> {
    // Get write lock
    // Schedule all the jobs and setup jobs_id
    // meant to be run once when we start the scheduler
    if let Ok(mut scheduler) = scheduler.write() {
        let jobs = (*scheduler).jobs.clone();
        for (_job_name, job) in jobs {
            let job = Arc::new(job.clone());
            schedule_job(job, &mut *scheduler)?;
        }
    } else {
        return Err(anyhow::anyhow!("Could not get write lock"));
    }
    return Ok(());
}

fn schedule_job(job: Arc<job::Job>, scheduler: &mut Scheduler) -> Result<(), anyhow::Error> {
    let job_name = (&*job).name.clone();
    let job_id = scheduler.job_scheduler.add(job_scheduler::Job::new(
        (&job.time).parse().unwrap(),
        closure!(move job, || {
            run_job(&*job);
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

async fn fetch_job_updates<'a>(scheduler: Arc<RwLock<Scheduler<'a>>>, role: Arc<RwLock<Role>>) -> () {
    // suppose to be run in a new thread
    // main method
    // loop
    // every x minutes, goes to the database
    // Acquires writer lock and unschedule any job that is needed and deletes from job_ids
    // Acquires writer lock and updates jobs.
    // schedule any new job
    // TODO: Should also check if we are still the leader
    loop {
        tick(scheduler.clone()); // This should be running in another thread
        thread::sleep(Duration::from_millis(4000));
        if let Ok(mut scheduler) = scheduler.write() {
            let last_updated_at = (*scheduler).last_updated_at;
            let disabled_jobs = match update_scheduler(&mut *scheduler).await {
                Ok(disabled_jobs) => disabled_jobs,
                _ => continue,
            };
            disable_jobs(&mut scheduler, disabled_jobs);
            let jobs = (*scheduler).jobs.clone();
            reschedule_jobs_if_needed(&mut scheduler, last_updated_at, jobs);
        }

        //This is terrible, but for now we also check here if we are still the leader
        // if not we should break and stop updating our scheduler
        if let Ok(role) = role.read() {
            match *role {
                Role::LEADER => println!("Still leader"),
                Role::FOLLOWER => break,
            };
        } // We don't need to check, we may act as a leader for longer than we should, but is ok for now
    }
}

fn reschedule_jobs_if_needed(scheduler: &mut RwLockWriteGuard<Scheduler>, last_updated_at: i64, jobs: HashMap<String, Job>) {
    for (_job_name, job) in jobs {
        if job.updated_at >= last_updated_at {
            let uuid = scheduler.job_ids.remove(&job.name);
            match uuid {
                Some(uuid) => scheduler.job_scheduler.remove(uuid),
                None => false,
            };

            let job = Arc::new(job.clone());
            schedule_job(job, &mut *scheduler); //TODO should check result
        }
    }
}

fn disable_jobs(scheduler: &mut RwLockWriteGuard<Scheduler>, disabled_jobs: Vec<Job>) {
    for job in disabled_jobs {
        let uuid = scheduler.job_ids.remove(&job.name);
        match uuid {
            Some(uuid) => scheduler.job_scheduler.remove(uuid),
            None => continue,
        };
    }
}

// Update the scheduler and return all jobs that were disabled
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
