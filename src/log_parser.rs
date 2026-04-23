use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TlsInfo {
    #[serde(default)]
    pub resumed: bool,
    #[serde(default)]
    pub version: u16,
    #[serde(default)]
    pub cipher_suite: u16,
    #[serde(default)]
    pub proto: String,
    #[serde(default)]
    pub server_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RequestInfo {
    pub remote_ip: String,
    #[serde(default)]
    pub remote_port: String,
    pub client_ip: String,
    pub proto: String,
    pub method: String,
    pub host: String,
    pub uri: String,
    #[serde(default)]
    pub headers: HashMap<String, Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<TlsInfo>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogEntry {
    pub ts: f64,
    pub request: RequestInfo,
    pub duration: f64,
    pub size: u64,
    pub status: u16,
    #[serde(default)]
    pub bytes_read: u64,
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub resp_headers: HashMap<String, Vec<String>>,
}

pub fn parse_log_file(path: &str) -> Vec<LogEntry> {
    let Ok(file) = std::fs::File::open(path) else {
        return Vec::new();
    };
    BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| serde_json::from_str(&line).ok())
        .collect()
}
