use chrono::Utc;
use sysinfo::{Disks, Networks, System};
use tokio::time;

use crate::buffer;
use crate::error::AppError;
use crate::models::{Buffer, MetricsConfig, MetricsSnapshot};

const BYTES_PER_MB: u64 = 1_048_576;
const BYTES_PER_GB: f64 = 1_073_741_824.0;

pub async fn run(config: MetricsConfig, buffer: Buffer) -> ! {
    let mut system = System::new();
    let mut disks = Disks::new_with_refreshed_list();
    let mut networks = Networks::new_with_refreshed_list();

    system.refresh_cpu_usage();
    system.refresh_memory();
    time::sleep(time::Duration::from_millis(200)).await;

    let mut interval = time::interval(time::Duration::from_secs(
        config.collect_interval_seconds,
    ));

    loop {
        interval.tick().await;

        system.refresh_cpu_usage();
        system.refresh_memory();
        disks.refresh(false);
        networks.refresh(false);

        let cpu_usage_percent = system.global_cpu_usage();
        let memory_used_mb = system.used_memory() / BYTES_PER_MB;
        let memory_total_mb = system.total_memory() / BYTES_PER_MB;

        let (disk_total_bytes, disk_available_bytes) =
            disks.iter().fold((0u64, 0u64), |(total, available), disk| {
                (
                    total.saturating_add(disk.total_space()),
                    available.saturating_add(disk.available_space()),
                )
            });
        let disk_used_bytes = disk_total_bytes.saturating_sub(disk_available_bytes);
        let disk_total_gb = disk_total_bytes as f64 / BYTES_PER_GB;
        let disk_used_gb = disk_used_bytes as f64 / BYTES_PER_GB;

        let (net_bytes_sent, net_bytes_received) =
            networks.iter().fold((0u64, 0u64), |(sent, received), (_, data)| {
                (
                    sent.saturating_add(data.total_transmitted()),
                    received.saturating_add(data.total_received()),
                )
            });

        let snapshot = MetricsSnapshot {
            cpu_usage_percent,
            memory_used_mb,
            memory_total_mb,
            disk_used_gb,
            disk_total_gb,
            net_bytes_sent,
            net_bytes_received,
            collected_at: Utc::now(),
        };

        if let Err(e) = buffer::set_metrics(&buffer, snapshot) {
            AppError::warn(&e);
        }
    }
}
