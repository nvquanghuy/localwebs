use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub port: u16,
    pub name: String,
    pub description: Option<String>,
    pub source: DetectionSource,
    pub url: String,
    pub title: Option<String>,
    pub server_header: Option<String>,
    pub process_name: Option<String>,
    pub pid: Option<u32>,
    pub is_healthy: bool,
    pub response_time_ms: Option<u64>,
    pub start_time: Option<i64>, // Unix timestamp of process start
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DetectionSource {
    Config,
    Process,
    Http,
    Pattern,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct OpenPort {
    pub port: u16,
    pub process_name: Option<String>,
    pub pid: Option<u32>,
    pub start_time: Option<i64>,
}

impl OpenPort {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            process_name: None,
            pid: None,
            start_time: None,
        }
    }

    pub fn with_process(port: u16, process_name: String, pid: Option<u32>) -> Self {
        Self {
            port,
            process_name: Some(process_name),
            pid,
            start_time: None,
        }
    }
}
