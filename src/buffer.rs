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
