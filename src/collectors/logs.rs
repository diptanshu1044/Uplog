use std::io::SeekFrom;

use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader};
use tokio::time;

use crate::buffer;
use crate::error::AppError;
use crate::models::{Buffer, LogLine, LogsConfig};

const RETRY_SLEEP_SECS: u64 = 2;
const POLL_INTERVAL_MS: u64 = 200;
const KEEPALIVE_SECS: u64 = 60;

pub async fn run(config: LogsConfig, buffer: Buffer) -> ! {
    for path in &config.paths {
        let path = path.clone();
        let buffer = buffer.clone();
        tokio::spawn(async move {
            loop {
                let mut file = loop {
                    match tokio::fs::File::open(&path).await {
                        Ok(f) => break f,
                        Err(e) => {
                            AppError::warn(&AppError::LogWatchError(format!(
                                "cannot open {path}: {e}"
                            )));
                            time::sleep(time::Duration::from_secs(RETRY_SLEEP_SECS)).await;
                        }
                    }
                };

                if file.seek(SeekFrom::End(0)).await.is_err() {
                    continue;
                }

                let reader = BufReader::new(file);
                let mut lines = reader.lines();

                loop {
                    match lines.next_line().await {
                        Ok(Some(line)) => {
                            let log_line = LogLine::from_file(&path, line);
                            if let Err(e) = buffer::push_log(&buffer, log_line) {
                                AppError::warn(&e);
                            }
                        }
                        Ok(None) => {
                            // TODO(v1.1): This branch cannot distinguish EOF-because-no-new-data
                            // from EOF-because-the-file-was-rotated (rename + create). If logrotate
                            // uses rename+create, the fd keeps pointing at the old inode and we poll
                            // here forever, missing the new file. Fix: compare inode/size on each
                            // poll via fs::metadata() and reopen if either changes.
                            time::sleep(time::Duration::from_millis(POLL_INTERVAL_MS)).await;
                        }
                        Err(_) => break,
                    }
                }
            }
        });
    }

    loop {
        time::sleep(time::Duration::from_secs(KEEPALIVE_SECS)).await;
    }
}
