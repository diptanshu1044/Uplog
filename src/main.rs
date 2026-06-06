mod buffer;
mod collectors;
mod config;
mod error;
mod models;
mod shipper;

use std::fs;
use std::io::{self, Write};

use clap::{CommandFactory, Parser, Subcommand};

use crate::error::AppError;
use crate::models::new_buffer;

#[derive(Parser)]
#[command(
    name = "uplog",
    version = env!("CARGO_PKG_VERSION"),
    about = "Lightweight log and metrics agent for Node.js applications",
    arg_required_else_help = true,
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the agent and ship logs and metrics
    Start {
        /// Path to config file (default: ./uplog.toml, ~/.uplog.toml or /etc/uplog/uplog.toml)
        #[arg(short, long)]
        config: Option<String>,
    },
    /// Interactively generate a config file at ~/.uplog.toml
    Init,
    /// Validate a config file and print the loaded values
    Check {
        /// Path to config file (default: ./uplog.toml, ~/.uplog.toml or /etc/uplog/uplog.toml)
        #[arg(short, long)]
        config: Option<String>,
    },
    /// Print the uplog version
    Version,
    /// Print this help text
    Help,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start { config } => run_start(config.as_deref()).await,
        Commands::Init => run_init(),
        Commands::Check { config } => run_check(config.as_deref()),
        Commands::Version => println!("uplog {}", env!("CARGO_PKG_VERSION")),
        Commands::Help => {
            Cli::command().print_help().ok();
            println!();
            std::process::exit(0);
        }
    }
}

// ─── start ──────────────────────────────────────────────────────────────────

async fn run_start(cli_config_path: Option<&str>) {
    let config = config::load(cli_config_path).unwrap_or_else(|e| e.exit());

    let agent_id = config.agent.id.clone();
    let buffer = new_buffer();

    tokio::spawn(collectors::logs::run(config.logs.clone(), buffer.clone()));

    tokio::spawn(collectors::metrics::run(
        config.metrics.clone(),
        buffer.clone(),
    ));

    tokio::spawn(shipper::run(config.shipper.clone(), buffer.clone(), agent_id));

    futures::future::pending::<()>().await
}

// ─── check ──────────────────────────────────────────────────────────────────

fn run_check(cli_config_path: Option<&str>) {
    let config = config::load(cli_config_path).unwrap_or_else(|e| e.exit());

    let loaded_from = config::resolve_config_path(cli_config_path)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "<unknown>".to_string());

    println!("Config valid. Loaded from: {loaded_from}");
    println!();
    println!("{:<19}{}", "agent id:", config.agent.id);
    println!("{:<19}{}", "endpoint:", config.shipper.endpoint);
    for (i, path) in config.logs.paths.iter().enumerate() {
        let label = if i == 0 { "log paths:" } else { "" };
        println!("{label:<19}{path}");
    }
    println!(
        "{:<19}{}s",
        "metric interval:", config.metrics.collect_interval_seconds
    );
    println!(
        "{:<19}{}s",
        "ship interval:", config.shipper.ship_interval_seconds
    );
}

// ─── init ─────────────────────────────────────────────────────────────────────

fn run_init() {
    println!("uplog init — let's create a config file.\n");

    let default_id = std::env::var("HOSTNAME").unwrap_or("my-server".to_string());
    let agent_id = prompt_with_default("Agent ID", &default_id);

    let endpoint = prompt_with_default("Backend endpoint", "http://localhost:3000/ingest");

    let paths = loop {
        let raw = prompt_with_default(
            "Log paths (comma-separated for multiple)",
            "~/.pm2/logs/",
        );
        let paths: Vec<String> = raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if paths.is_empty() {
            println!("Please enter at least one log path.");
            continue;
        }
        break paths;
    };

    let metric_interval = prompt_u64("Metric interval (seconds)", 30);
    let ship_interval = prompt_u64("Ship interval (seconds)", 60);

    let home = dirs::home_dir().unwrap_or_else(|| {
        AppError::InitError("could not determine home directory".to_string()).exit()
    });
    let out_path = home.join(".uplog.toml");

    if out_path.exists() {
        print!("~/.uplog.toml already exists. Overwrite? [y/N]: ");
        let answer = read_line().to_lowercase();
        if answer != "y" && answer != "yes" {
            println!("Aborted.");
            std::process::exit(0);
        }
    }

    let paths_block = paths
        .iter()
        .map(|p| format!("  \"{p}\""))
        .collect::<Vec<_>>()
        .join(",\n");

    let contents = format!(
        "[agent]\n\
         id = \"{agent_id}\"\n\
         \n\
         [logs]\n\
         paths = [\n{paths_block}\n]\n\
         \n\
         [metrics]\n\
         collect_interval_seconds = {metric_interval}\n\
         \n\
         [shipper]\n\
         endpoint = \"{endpoint}\"\n\
         ship_interval_seconds = {ship_interval}\n"
    );

    if let Err(e) = fs::write(&out_path, contents) {
        AppError::InitError(format!("failed to write {}: {e}", out_path.display())).exit();
    }

    let written_path = out_path.to_string_lossy().to_string();
    match config::load(Some(&written_path)) {
        Ok(_) => {
            println!("\nConfig written to ~/.uplog.toml");
            println!("\nRun:  pm2 start uplog -- start");
        }
        Err(e) => {
            AppError::warn(&e);
            println!("Please edit ~/.uplog.toml manually to fix the issue.");
        }
    }
}

// ─── prompt helpers ─────────────────────────────────────────────────────────

fn read_line() -> String {
    io::stdout().flush().ok();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        AppError::InitError("failed to read input from stdin".to_string()).exit();
    }
    input.trim().to_string()
}

fn prompt_with_default(question: &str, default: &str) -> String {
    print!("{question} [{default}]: ");
    let input = read_line();
    if input.is_empty() {
        default.to_string()
    } else {
        input
    }
}

fn prompt_u64(question: &str, default: u64) -> u64 {
    let default_str = default.to_string();
    loop {
        let raw = prompt_with_default(question, &default_str);
        match raw.parse::<u64>() {
            Ok(value) if value > 0 => return value,
            Ok(_) => println!("Value must be greater than 0."),
            Err(_) => println!("Please enter a valid whole number."),
        }
    }
}
