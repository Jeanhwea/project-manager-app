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
}

pub type Result<T> = std::result::Result<T, GitLabError>;
