//! Configuration domain module
//!
//! This module contains configuration management and type-safe access.

pub mod manager;
pub mod schema;

pub use schema::AppConfig;

/// Configuration-specific error type
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Configuration file not found: {0}")]
    FileNotFound(String),

    #[error("Parse error: {0}")]
    ParseError(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Configuration manager trait
pub trait ConfigManager {
    fn load() -> Result<AppConfig>;
    fn save(&self, config: &AppConfig) -> Result<()>;
}

/// Common result type for configuration operations
pub type Result<T> = std::result::Result<T, ConfigError>;

// Re-export manager types
pub use manager::MultiSourceConfigManager;

/// Default configuration manager implementation
pub type DefaultConfigManager = MultiSourceConfigManager;
