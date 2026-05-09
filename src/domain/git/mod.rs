pub mod command;
pub mod remote;
pub mod repository;
pub mod url_parser;

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("Git command failed: {0}")]
    CommandFailed(String),

    #[error("Invalid remote URL: {0}")]
    InvalidRemoteUrl(String),

    #[error("Remote not found: {0}")]
    RemoteNotFound(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),
}

#[derive(Debug, Clone, PartialEq)]
pub enum GitProtocol {
    Ssh,
    Http,
    Https,
    Git,
}

pub type Result<T> = std::result::Result<T, GitError>;
