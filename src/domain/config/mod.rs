//! 配置管理
//!
//! 所有持久化配置统一存放在 `~/.pma/` 目录下（可通过 `$PMA_CONFIG_DIR` 覆盖）。
//!
//! - `config.toml`  — 主配置 (repository, remote, sync)
//! - `gitlab.toml`  — GitLab 服务器凭据

pub mod manager;
pub mod schema;

pub use manager::ConfigDir;
pub use schema::GitLabServer;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Configuration file not found: {0}")]
    FileNotFound(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ConfigError>;
