use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Job {
    pub name: String,
    pub time: String,
    pub job_type: i32,
    pub script: String,
    pub timeout: i32,
    pub active: bool,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Execution {
    pub start_time: i64,
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
