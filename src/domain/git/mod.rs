pub mod command;
pub mod remote;
pub mod repository;

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("Repository not found: {0}")]
    #[allow(dead_code)]
    RepositoryNotFound(String),

    #[error("Git command failed: {0}")]
    CommandFailed(String),

    #[error("Invalid remote URL: {0}")]
    InvalidRemoteUrl(String),

    #[error("Working directory not clean")]
    #[allow(dead_code)]
    WorkdirNotClean,

    #[error("Branch not found: {0}")]
    #[allow(dead_code)]
    BranchNotFound(String),

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

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum RepositoryStatus {
    Clean,
    Dirty,
    UnpushedCommits,
    Diverged,
    Unknown,
}

pub type Result<T> = std::result::Result<T, GitError>;
