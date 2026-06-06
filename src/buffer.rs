use std::mem;

use crate::error::AppError;
use crate::models::{Buffer, LogLine, MetricsSnapshot};

pub fn push_log(buffer: &Buffer, line: LogLine) -> Result<(), AppError> {
    let mut inner = buffer
        .lock()
        .map_err(|_| AppError::BufferLockError)?;
    inner.logs.push(line);
    Ok(())
}

pub fn set_metrics(buffer: &Buffer, snapshot: MetricsSnapshot) -> Result<(), AppError> {
    let mut inner = buffer
        .lock()
        .map_err(|_| AppError::BufferLockError)?;
    inner.metrics = Some(snapshot);
    Ok(())
}

pub fn drain(buffer: &Buffer) -> Result<(Vec<LogLine>, Option<MetricsSnapshot>), AppError> {
    let mut inner = buffer
        .lock()
        .map_err(|_| AppError::BufferLockError)?;
    let logs = mem::take(&mut inner.logs);
    let metrics = inner.metrics.take();
    Ok((logs, metrics))
}

pub fn is_empty(buffer: &Buffer) -> Result<bool, AppError> {
    let inner = buffer
        .lock()
        .map_err(|_| AppError::BufferLockError)?;
    Ok(inner.logs.is_empty() && inner.metrics.is_none())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{LogLine, MetricsSnapshot, new_buffer};
    use chrono::Utc;

    #[test]
    fn push_log_adds_to_vec() {
        let buffer = new_buffer();
        let line = LogLine::from_file("/var/log/app.log", "test message".to_string());
        push_log(&buffer, line).expect("push_log failed");

        let (logs, _metrics) = drain(&buffer).expect("drain failed");
        assert_eq!(logs.len(), 1);
        assert!(is_empty(&buffer).expect("is_empty failed"));
    }

    #[test]
    fn set_metrics_overwrites_previous() {
        let buffer = new_buffer();
        let first = MetricsSnapshot {
            cpu_usage_percent: 10.0,
            memory_used_mb: 0,
            memory_total_mb: 0,
            disk_used_gb: 0.0,
            disk_total_gb: 0.0,
            net_bytes_sent: 0,
            net_bytes_received: 0,
            collected_at: Utc::now(),
        };
        let second = MetricsSnapshot {
            cpu_usage_percent: 50.0,
            memory_used_mb: 0,
            memory_total_mb: 0,
            disk_used_gb: 0.0,
            disk_total_gb: 0.0,
            net_bytes_sent: 0,
            net_bytes_received: 0,
            collected_at: Utc::now(),
        };
        set_metrics(&buffer, first).expect("set_metrics failed");
        set_metrics(&buffer, second).expect("set_metrics failed");

        let (_logs, metrics) = drain(&buffer).expect("drain failed");
        let snapshot = metrics.expect("expected metrics snapshot");
        assert_eq!(snapshot.cpu_usage_percent, 50.0);
    }

    #[test]
    fn drain_clears_buffer() {
        let buffer = new_buffer();
        push_log(
            &buffer,
            LogLine::from_file("/var/log/a.log", "line one".to_string()),
        )
        .expect("push_log failed");
        push_log(
            &buffer,
            LogLine::from_file("/var/log/b.log", "line two".to_string()),
        )
        .expect("push_log failed");
        set_metrics(
            &buffer,
            MetricsSnapshot {
                cpu_usage_percent: 0.0,
                memory_used_mb: 0,
                memory_total_mb: 0,
                disk_used_gb: 0.0,
                disk_total_gb: 0.0,
                net_bytes_sent: 0,
                net_bytes_received: 0,
                collected_at: Utc::now(),
            },
        )
        .expect("set_metrics failed");

        let (logs, metrics) = drain(&buffer).expect("drain failed");
        assert_eq!(logs.len(), 2);
        assert!(metrics.is_some());

        let (logs, metrics) = drain(&buffer).expect("drain failed");
        assert!(logs.is_empty());
        assert!(metrics.is_none());
    }

    #[test]
    fn is_empty_reflects_state() {
        let buffer = new_buffer();
        assert!(is_empty(&buffer).expect("is_empty failed"));

        push_log(
            &buffer,
            LogLine::from_file("/var/log/app.log", "hello".to_string()),
        )
        .expect("push_log failed");
        assert!(!is_empty(&buffer).expect("is_empty failed"));

        drain(&buffer).expect("drain failed");
        assert!(is_empty(&buffer).expect("is_empty failed"));
    }

    #[test]
    fn drain_with_no_metrics_returns_none() {
        let buffer = new_buffer();
        push_log(
            &buffer,
            LogLine::from_file("/var/log/app.log", "only logs".to_string()),
        )
        .expect("push_log failed");

        let (logs, metrics) = drain(&buffer).expect("drain failed");
        assert_eq!(logs.len(), 1);
        assert!(metrics.is_none());
    }
}
