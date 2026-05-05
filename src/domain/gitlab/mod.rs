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

#[derive(Debug, Clone, Default)]
pub struct GitLabConfig {
    pub server: Option<String>,
    pub token: Option<String>,
}

pub type Result<T> = std::result::Result<T, GitLabError>;
