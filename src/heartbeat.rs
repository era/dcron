use serde::{Deserialize, Serialize};


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Heartbeat {
    pub server: String,
    pub timestamp: i64,
}
