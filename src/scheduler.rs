use crate::{db, job::Job};
use std::collections::HashMap;
// job_scheduler crate https://docs.rs/job_scheduler/1.2.1/job_scheduler/
use job_scheduler;

// This probably should be another binary
pub struct Scheduler {
    // Holds the main Job struct
    jobs: HashMap<String, Job>,
    // Used to unschedule a job if needed
    job_ids: HashMap<String, job_scheduler::Uuid>,
    // Used to request to the database only jobs created after it
    last_updated_at: i64,
}

impl Scheduler {
    // Get all the jobs in the database and updates it every 5 min
    // Schedule the jobs using job_scheduler and keeps their uuid
    // when updating the jobs, we need to hold a write lock
    // the job thread should request read lock, and send the job to a worker
}
