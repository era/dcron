use crate::{config::Config, db, job::Job};
use std::collections::HashMap;
// job_scheduler crate https://docs.rs/job_scheduler/1.2.1/job_scheduler/
use anyhow;
use job_scheduler;
use std::sync::{Arc, RwLock};

// Maybe should use an Arc on the Scheduler itself
pub struct Scheduler {
    // Holds the main Job struct
    jobs: Arc<RwLock<HashMap<String, Job>>>,
    // Used to unschedule a job if needed
    job_ids: Arc<RwLock<HashMap<String, job_scheduler::Uuid>>>,
    // Used to request to the database only jobs created after it
    last_updated_at: i64,
}

impl Scheduler {
    // Get all the jobs in the database and updates it every 5 min
    // Schedule the jobs using job_scheduler and keeps their uuid
    // when updating the jobs, we need to hold a write lock
    // the job thread should request read lock, and send the job to a worker

    pub async fn new() -> Result<Scheduler, anyhow::Error> {
        // Gets all the jobs from the database and set jobs
        // Creates the new object
        return Err(anyhow::anyhow!("Opss"));
    }

    pub fn schedule_all(mut self: &Self) -> Result<(), anyhow::Error> {
        // Get read lock
        // Schedule all the jobs and setup jobs_id
        // meant to be run once when we start the scheduler
        return Ok(());
    }

    pub fn update_schedules(mut self: &Self) -> Result<(), anyhow::Error> {
        // main method
        // loop
        // every 5 minutes, goes to the database
        // Acquires writer lock and unschedule any job that is needed and deletes from job_ids
        // Acquires writer lock and updates jobs.
        // schedule any new job
        return Ok(());
    }

    fn run_job(mut self: &Self, job: &Job) -> Result<(), anyhow::Error> {
        // Acquires reader lock on job ids (not sure if we really need)
        // If standalone runs the script
        // Otherwise gets a worker IP and sends an execution request to it
        Ok(())
    }
}
