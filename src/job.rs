use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Job {
    pub name: String,
    pub time: String,
    pub job_type: i32,
    pub script: String,
    pub timeout: i32,
    pub active: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Execution {
    pub start_time: u64,
    pub log: String,
    pub status: Status,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Status {
    RUNNING,
    TIMEOUT,
    FAILED,
    SUCCEEDED,
}
