use std::fs;
use std::path::PathBuf;

use crate::error::AppError;
use crate::models::Config;

const LOCAL_CONFIG: &str = "./uplog.toml";
const SYSTEM_CONFIG: &str = "/etc/uplog/uplog.toml";

pub fn load(cli_path: Option<&str>) -> Result<Config, AppError> {
    let path = resolve_config_path(cli_path).ok_or(AppError::ConfigNotFound)?;

    let contents = fs::read_to_string(&path).map_err(|e| {
        AppError::ConfigParseError(format!(
            "failed to read {}: {e}",
            path.display()
        ))
    })?;

    let mut config: Config = toml::from_str(&contents)
        .map_err(|e| AppError::ConfigParseError(e.to_string()))?;

    validate(&config)?;

    config.shipper.endpoint = config.agent.backend_url.clone();

    Ok(config)
}

fn resolve_config_path(cli_path: Option<&str>) -> Option<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(path) = cli_path {
        candidates.push(PathBuf::from(path));
    }
    candidates.push(PathBuf::from(LOCAL_CONFIG));
    candidates.push(PathBuf::from(SYSTEM_CONFIG));

    candidates.into_iter().find(|path| path.exists())
}

fn validate(config: &Config) -> Result<(), AppError> {
    if config.agent.id.trim().is_empty() {
        return Err(AppError::ConfigValidationError(
            "agent.id cannot be empty".to_string(),
        ));
    }

    if config.agent.backend_url.is_empty() {
        return Err(AppError::ConfigValidationError(
            "agent.backend_url must not be empty".into(),
        ));
    }

    if !config.agent.backend_url.starts_with("http://")
        && !config.agent.backend_url.starts_with("https://")
    {
        return Err(AppError::ConfigValidationError(
            "agent.backend_url must start with \"http://\" or \"https://\"".into(),
        ));
    }

    if config.agent.api_key.is_empty() {
        return Err(AppError::ConfigValidationError(
            "agent.api_key must not be empty".into(),
        ));
    }

    if config.logs.paths.is_empty() {
        return Err(AppError::ConfigValidationError(
            "logs.paths must have at least one entry".into(),
        ));
    }

    if config.metrics.collect_interval_seconds == 0 {
        return Err(AppError::ConfigValidationError(
            "metrics.collect_interval_seconds must be greater than 0".into(),
        ));
    }

    if config.shipper.ship_interval_seconds == 0 {
        return Err(AppError::ConfigValidationError(
            "shipper.ship_interval_seconds must be greater than 0".into(),
        ));
    }

    Ok(())
}
