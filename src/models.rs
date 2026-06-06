use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ─── Config ───────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub agent: AgentConfig,
    pub logs: LogsConfig,
    pub metrics: MetricsConfig,
    pub shipper: ShipperConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AgentConfig {
    pub id: String,
    pub backend_url: String,
    pub api_key: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LogsConfig {
    pub paths: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MetricsConfig {
    pub collect_interval_seconds: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ShipperConfig {
    #[serde(skip, default)]
    pub endpoint: String,
    pub ship_interval_seconds: u64,
    pub api_key: Option<String>,
}

// ─── Log ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct LogLine {
    pub source: String,
    pub line: String,
    pub timestamp: DateTime<Utc>,
}

impl LogLine {
    pub fn from_file(path: &str, line: String) -> Self {
        Self {
            source: format!("file:{}", path),
            line,
            timestamp: Utc::now(),
        }
    }
}

// ─── Metrics ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct MetricsSnapshot {
    pub cpu_usage_percent: f32,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub disk_used_gb: f64,
    pub disk_total_gb: f64,
    pub net_bytes_sent: u64,
    pub net_bytes_received: u64,
    pub collected_at: DateTime<Utc>,
}

// ─── Buffer ───────────────────────────────────────────────────────────────────

use std::sync::{Arc, Mutex};

#[derive(Debug, Default)]
pub struct BufferInner {
    pub logs: Vec<LogLine>,
    pub metrics: Option<MetricsSnapshot>,
}

pub type Buffer = Arc<Mutex<BufferInner>>;

pub fn new_buffer() -> Buffer {
    Arc::new(Mutex::new(BufferInner::default()))
}

// ─── Payload ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct Payload {
    pub agent_id: String,
    pub timestamp: DateTime<Utc>,
    pub metrics: Option<MetricsSnapshot>,
    pub logs: Vec<LogLine>,
}
