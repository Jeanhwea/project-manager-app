//! Git domain module
//!
//! This module contains Git repository abstractions and operations.

pub mod command;
pub mod remote;
pub mod repository;

/// Git-specific error type
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("Repository not found: {0}")]
    RepositoryNotFound(String),
    
    #[error("Git command failed: {0}")]
    CommandFailed(String),
    
    #[error("Invalid remote URL: {0}")]
    InvalidRemoteUrl(String),
    
    #[error("Working directory not clean")]
    WorkdirNotClean,
    
    #[error("Branch not found: {0}")]
    BranchNotFound(String),
    
    #[error("Remote not found: {0}")]
    RemoteNotFound(String),
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),
}

/// Git protocol type
#[derive(Debug, Clone, PartialEq)]
pub enum GitProtocol {
    Ssh,
    Http,
    Https,
    Git,
}

/// Git repository status
#[derive(Debug, Clone, PartialEq)]
pub enum RepositoryStatus {
    Clean,
    Dirty,
    UnpushedCommits,
    Diverged,
    Unknown,
}

/// Common result type for Git operations
pub type Result<T> = std::result::Result<T, GitError>;