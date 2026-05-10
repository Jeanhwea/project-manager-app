mod command;
pub mod repository;

pub use command::GitCommandRunner;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, GitError>;

#[derive(Debug, Error)]
pub enum GitError {
    #[error("{0}")]
    CommandFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
