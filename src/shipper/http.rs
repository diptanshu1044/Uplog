use chrono::Utc;
use reqwest::Client;
use tokio::time::{interval, Duration};

use crate::buffer;
use crate::error::AppError;
use crate::models::{Buffer, Payload, ShipperConfig};

const REQUEST_TIMEOUT_SECS: u64 = 10;
const RETRY_DELAY_SECS: u64 = 5;

pub async fn run(config: ShipperConfig, buffer: Buffer, agent_id: String) -> ! {
    let client = match Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
    {
        Ok(c) => c,
        Err(e) => AppError::ShipError(format!("failed to build HTTP client: {e}")).exit(),
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

        let mut shipped = false;
        for attempt in 1..=3 {
            let mut req = client
                .post(&config.endpoint)
                .header("Content-Type", "application/json")
                .body(json_string.clone());

            if let Some(key) = &config.api_key {
                req = req.header("Authorization", format!("Bearer {}", key));
            }

            match req.send().await {
                Ok(response) if response.status().is_success() => {
                    shipped = true;
                    break;
                }
                Ok(response) => {
                    AppError::warn(&AppError::ShipError(format!(
                        "attempt {attempt}: POST to {} returned {}",
                        config.endpoint,
                        response.status()
                    )));
                }
                Err(e) => {
                    AppError::warn(&AppError::ShipError(format!(
                        "attempt {attempt}: POST to {} failed: {e}",
                        config.endpoint
                    )));
                }
            }

            if attempt < 3 {
                tokio::time::sleep(Duration::from_secs(RETRY_DELAY_SECS)).await;
            }
        }

        if !shipped {
            AppError::warn(&AppError::ShipError(
                "batch dropped after 3 attempts".into(),
            ));
        }
    }
}
