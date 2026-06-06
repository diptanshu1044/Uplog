use std::fmt;
use std::process;

#[derive(Debug)]
pub enum AppError {
    ConfigNotFound,
    ConfigParseError(String),
    ConfigValidationError(String),
    LogWatchError(String),
    MetricsError(String),
    ShipError(String),
    BufferLockError,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::ConfigNotFound => write!(
                f,
                "[uplog error] config: no config file found (checked ./uplog.toml, \
                 /etc/uplog/uplog.toml)"
            ),
            AppError::ConfigParseError(msg) => {
                write!(f, "[uplog error] config: parse failed — {msg}")
            }
            AppError::ConfigValidationError(msg) => {
                write!(f, "[uplog error] config: invalid field — {msg}")
            }
            AppError::LogWatchError(msg) => {
                write!(f, "[uplog error] logs: {msg}")
            }
            AppError::MetricsError(msg) => {
                write!(f, "[uplog error] metrics: {msg}")
            }
            AppError::ShipError(msg) => {
                write!(f, "[uplog error] shipper: {msg}")
            }
            AppError::BufferLockError => write!(
                f,
                "[uplog error] buffer: failed to acquire lock on shared buffer"
            ),
        }
    }
}

impl std::error::Error for AppError {}

impl AppError {
    pub fn exit(self) -> ! {
        eprintln!("{self}");
        process::exit(1);
    }

    pub fn exit_ref(e: &AppError) -> ! {
        eprintln!("{e}");
        process::exit(1);
    }

    pub fn warn(e: &AppError) {
        eprintln!("{e}");
    }
}
