pub mod auth;
pub mod client;
pub mod models;

#[derive(Debug, thiserror::Error)]
pub enum GitLabError {
    #[error("Network error: {0}")]
    NetworkError(#[from] Box<ureq::Error>),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Rate limited")]
    RateLimited,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// GitLab API 客户端使用的内部配置（非持久化配置）
#[derive(Debug, Clone)]
pub struct GitLabConfig {
    pub server: Option<String>,
    pub token: Option<String>,
    #[allow(dead_code)]
    pub default_protocol: CloneProtocol,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum CloneProtocol {
    Ssh,
    Http,
    Https,
}

impl Default for GitLabConfig {
    fn default() -> Self {
        Self {
            server: None,
            token: None,
            default_protocol: CloneProtocol::Https,
        }
    }
}

pub type Result<T> = std::result::Result<T, GitLabError>;
