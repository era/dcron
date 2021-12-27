use crate::{
    config::Config,
    db,
    job::{self, Job},
};
use std::{collections::HashMap, sync::RwLockWriteGuard};
// job_scheduler crate https://docs.rs/job_scheduler/1.2.1/job_scheduler/
use anyhow;
use closure::closure;
use job_scheduler::{self, Schedule};
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use std::sync::{Arc, RwLock};

// Maybe should use an Arc on the Scheduler itself
pub struct Scheduler<'a> {
    // Holds the main Job struct
    jobs: HashMap<String, Job>,
    // Used to unschedule a job if needed
    job_ids: HashMap<String, job_scheduler::Uuid>,
    // Used to request to the database only jobs created after it
    last_updated_at: i64,
    job_scheduler: job_scheduler::JobScheduler<'a>,
}

// Get all the jobs in the database and updates it every 5 min
// Schedule the jobs using job_scheduler and keeps their uuid
// when updating the jobs, we need to hold a write lock
// the job thread should request read lock, and send the job to a worker

pub async fn new() -> Result<(), anyhow::Error> {
    // Gets all the jobs from the database and set jobs
    // Creates the new object
    return Err(anyhow::anyhow!("Opss"));
}

pub fn schedule_all(scheduler: Arc<RwLock<Scheduler>>) -> Result<(), anyhow::Error> {
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

pub fn update_schedules(scheduler: Arc<RwLock<Scheduler>>) -> ! {
    // main method
    // loop
    // every 5 minutes, goes to the database
    // Acquires writer lock and unschedule any job that is needed and deletes from job_ids
    // Acquires writer lock and updates jobs.
    // schedule any new job
    loop {}
}

fn run_job(job: &Job) -> Result<(), anyhow::Error> {
    // Acquires reader lock on job ids (not sure if we really need)
    // If standalone runs the script
    // Otherwise gets a worker IP and sends an execution request to it
    Ok(())
}
