use crate::{
    config::Config,
    db,
    job::{self, Job},
};
use std::collections::HashMap;
use std::env;
// job_scheduler crate https://docs.rs/job_scheduler/1.2.1/job_scheduler/
use anyhow;
use closure::closure;
use job_scheduler::Schedule;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use tokio::task;

// Maybe should use an Arc on the Scheduler itself
pub struct Scheduler<'a> {
    // Holds the main Job struct
    jobs: HashMap<String, Job>,
    // Used to unschedule a job if needed
    job_ids: HashMap<String, job_scheduler::Uuid>,
    // Used to request to the database only jobs created after it
    last_updated_at: i64,
    job_scheduler: job_scheduler::JobScheduler<'a>,
    config: Config,
}

// Get all the jobs in the database and updates it every 5 min
// Schedule the jobs using job_scheduler and keeps their uuid
// when updating the jobs, we need to hold a write lock
// the job thread should request read lock, and send the job to a worker

pub async fn run() -> ! {
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

    let db = match db::get_db(&config).await {
        Ok(db) => db,
        _ => panic!("Could not get DB"),
    };

    let jobs = match db.find_all_active().await {
        Ok(jobs) => jobs,
        _ => panic!("Could not initialise jobs"),
    };

    let mut scheduler = Scheduler {
        jobs: HashMap::new(),
        job_ids: HashMap::new(),
        last_updated_at: 0,
        job_scheduler: job_scheduler::JobScheduler::new(),
        config: config,
    };

    for job in jobs {
        scheduler.jobs.insert(job.name.clone(), job);
    }

    let arc_scheduler = Arc::new(RwLock::new(scheduler));

    schedule_all(arc_scheduler.clone()).unwrap();

    update_schedules(arc_scheduler.clone()); // This runs in a loop
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

fn schedule_job(job: Arc<Job>, scheduler: &mut Scheduler) -> Result<(), anyhow::Error> {
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

fn update_schedules(scheduler: Arc<RwLock<Scheduler>>) -> ! {
    // suppose to be run in a new thread
    // main method
    // loop
    // every x minutes, goes to the database
    // Acquires writer lock and unschedule any job that is needed and deletes from job_ids
    // Acquires writer lock and updates jobs.
    // schedule any new job
    loop {
        tick(scheduler.clone()); // This should be running in another thread
        thread::sleep(Duration::from_millis(4000));
        if let Ok(mut scheduler) = scheduler.write() {
            let last_updated_at = (*scheduler).last_updated_at;
            let disabled_jobs = match update_scheduler(&mut *scheduler) {
                Ok(disabled_jobs) => disabled_jobs,
                _ => continue,
            };
            for job in disabled_jobs {
                let uuid = scheduler.job_ids.remove(&job.name);
                match uuid {
                    Some(uuid) => scheduler.job_scheduler.remove(uuid),
                    None => continue,
                };
            }
            let jobs = (*scheduler).jobs.clone();
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
    }
}

// Update the scheduler and return all jobs that were disabled
fn update_scheduler(scheduler: &mut Scheduler) -> Result<Vec<Job>, anyhow::Error> {
    Ok(vec![])
}

fn run_job(job: &Job) -> Result<(), anyhow::Error> {
    // Acquires reader lock on job ids (not sure if we really need)
    // If standalone runs the script
    // Otherwise gets a worker IP and sends an execution request to it
    Ok(())
}
