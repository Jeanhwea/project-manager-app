mod command;
mod context;
mod diagnose;
pub mod release;
mod remote;
pub mod repository;

pub use command::GitCommandRunner;
pub use context::collect_context;
pub use diagnose::diagnose_repo;
pub use release::{ReleaseGitState, switch_to_git_root, validate_git_state};

use thiserror::Error;

pub type Result<T> = std::result::Result<T, GitError>;

#[derive(Debug, Error)]
pub enum GitError {
    #[error("{0}")]
    CommandFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
