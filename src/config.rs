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

    let config: Config = toml::from_str(&contents)
        .map_err(|e| AppError::ConfigParseError(e.to_string()))?;

    validate(&config)?;

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

    if config.shipper.endpoint.trim().is_empty() {
        return Err(AppError::ConfigValidationError(
            "shipper.endpoint must not be empty".into(),
        ));
    }

    if config.shipper.ship_interval_seconds == 0 {
        return Err(AppError::ConfigValidationError(
            "shipper.ship_interval_seconds must be greater than 0".into(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;
    use crate::models::{AgentConfig, Config, LogsConfig, MetricsConfig, ShipperConfig};

    fn valid_config() -> Config {
        Config {
            agent: AgentConfig {
                id: "test-agent".to_string(),
            },
            logs: LogsConfig {
                paths: vec!["/var/log/app.log".to_string()],
            },
            metrics: MetricsConfig {
                collect_interval_seconds: 10,
            },
            shipper: ShipperConfig {
                endpoint: "http://localhost:9000".to_string(),
                ship_interval_seconds: 5,
                api_key: None,
            },
        }
    }

    #[test]
    fn valid_config_passes_validation() {
        assert!(validate(&valid_config()).is_ok());
    }

    #[test]
    fn empty_agent_id_fails() {
        let mut config = valid_config();
        config.agent.id = String::new();
        assert!(matches!(
            validate(&config),
            Err(AppError::ConfigValidationError(_))
        ));
    }

    #[test]
    fn whitespace_only_agent_id_fails() {
        let mut config = valid_config();
        config.agent.id = "   ".to_string();
        assert!(matches!(
            validate(&config),
            Err(AppError::ConfigValidationError(_))
        ));
    }

    #[test]
    fn empty_paths_fails() {
        let mut config = valid_config();
        config.logs.paths = vec![];
        assert!(matches!(
            validate(&config),
            Err(AppError::ConfigValidationError(_))
        ));
    }

    #[test]
    fn zero_metrics_interval_fails() {
        let mut config = valid_config();
        config.metrics.collect_interval_seconds = 0;
        assert!(matches!(
            validate(&config),
            Err(AppError::ConfigValidationError(_))
        ));
    }

    #[test]
    fn empty_endpoint_fails() {
        let mut config = valid_config();
        config.shipper.endpoint = String::new();
        assert!(matches!(
            validate(&config),
            Err(AppError::ConfigValidationError(_))
        ));
    }

    #[test]
    fn zero_ship_interval_fails() {
        let mut config = valid_config();
        config.shipper.ship_interval_seconds = 0;
        assert!(matches!(
            validate(&config),
            Err(AppError::ConfigValidationError(_))
        ));
    }
}
