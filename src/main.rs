mod buffer;
mod collectors;
mod config;
mod error;
mod models;
mod shipper;

use clap::Parser;

use crate::models::new_buffer;

#[derive(Parser)]
#[command(name = "uplog", version, about = "Lightweight log and metrics agent")]
struct Cli {
    /// Path to config file (default: ./uplog.toml or /etc/uplog/uplog.toml)
    #[arg(short, long)]
    config: Option<String>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let config = config::load(cli.config.as_deref())
        .unwrap_or_else(|e| e.exit());

    let agent_id = config.agent.id.clone();
    let buffer = new_buffer();

    tokio::spawn(collectors::logs::run(
        config.logs.clone(),
        buffer.clone(),
    ));

    tokio::spawn(collectors::metrics::run(
        config.metrics.clone(),
        buffer.clone(),
    ));

    tokio::spawn(shipper::run(
        config.shipper.clone(),
        buffer.clone(),
        agent_id,
    ));

    futures::future::pending::<()>().await
}
