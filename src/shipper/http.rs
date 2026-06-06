use chrono::Utc;
use reqwest::Client;
use tokio::time::{interval, Duration};

use crate::buffer;
use crate::error::AppError;
use crate::models::{Buffer, Payload, ShipperConfig};

const REQUEST_TIMEOUT_SECS: u64 = 10;

pub async fn run(config: ShipperConfig, buffer: Buffer, agent_id: String) -> ! {
    let client = match Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
    {
        Ok(c) => c,
        Err(e) => AppError::exit(&AppError::ShipError(
          format!("failed to build HTTP client: {e}")
        )),
    };

    let mut ticker = interval(Duration::from_secs(config.ship_interval_seconds));
    ticker.tick().await; // discard the immediate first tick — wait a full interval before first ship

    loop {
        ticker.tick().await;

        match buffer::is_empty(&buffer) {
            Ok(true) => continue,
            Ok(false) => {}
            Err(e) => {
                AppError::warn(&e);
                continue;
            }
        }

        let (log_lines, metrics_snapshot) = match buffer::drain(&buffer) {
            Ok(data) => data,
            Err(e) => {
                AppError::warn(&e);
                continue;
            }
        };

        let payload = Payload {
            agent_id: agent_id.clone(),
            timestamp: Utc::now(),
            metrics: metrics_snapshot,
            logs: log_lines,
        };

        let json_string = match serde_json::to_string(&payload) {
            Ok(s) => s,
            Err(e) => {
                AppError::warn(&AppError::ShipError(format!(
                    "failed to serialize payload: {e}"
                )));
                continue;
            }
        };

        // MVP: failed batches are dropped. No disk persistence or retry queue
        // until v1.1. If the backend is down, data is lost for that interval.
        let response = match client
            .post(&config.endpoint)
            .header("Content-Type", "application/json")
            .body(json_string)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                AppError::warn(&AppError::ShipError(format!(
                    "POST to {} failed: {e}",
                    config.endpoint
                )));
                continue;
            }
        };

        if !response.status().is_success() {
            AppError::warn(&AppError::ShipError(format!(
                "POST to {} returned {}",
                config.endpoint,
                response.status()
            )));
        }
    }
}
