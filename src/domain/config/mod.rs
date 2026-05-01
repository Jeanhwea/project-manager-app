//! Configuration domain module
//!
//! All persistent configuration lives under `~/.pma/` (or `$PMA_CONFIG_DIR`).
//!
//! Files:
//! - `config.toml`  — main application config (repository, remote, sync)
//! - `gitlab.toml`  — GitLab server credentials

pub mod manager;
pub mod schema;

pub use manager::ConfigDir;
pub use schema::GitLabServer;

/// Configuration-specific error type
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Configuration file not found: {0}")]
    FileNotFound(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Common result type for configuration operations
pub type Result<T> = std::result::Result<T, ConfigError>;
