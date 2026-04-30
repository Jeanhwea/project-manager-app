//! GitLab domain module
//!
//! This module contains GitLab API integration and data models.

pub mod auth;
pub mod client;
pub mod models;

/// GitLab-specific error type
#[derive(Debug, thiserror::Error)]
pub enum GitLabError {
    #[error("Network error: {0}")]
    NetworkError(#[from] ureq::Error),
    
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

/// GitLab clone protocol
#[derive(Debug, Clone, PartialEq)]
pub enum CloneProtocol {
    Ssh,
    Http,
    Https,
}

/// GitLab API configuration
#[derive(Debug, Clone)]
pub struct GitLabConfig {
    pub server: Option<String>,
    pub token: Option<String>,
    pub default_protocol: CloneProtocol,
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

/// Common result type for GitLab operations
pub type Result<T> = std::result::Result<T, GitLabError>;