use crate::domain::editor::EditorError;
use crate::domain::git::GitError;
use crate::domain::gitlab::GitLabError;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Command not available: {0}")]
    CommandNotAvailable(String),

    #[error("{0}")]
    Editor(#[from] EditorError),

    #[error("{0}")]
    Git(#[from] GitError),

    #[error("{0}")]
    GitLab(#[from] GitLabError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    NotFound(String),

    #[error("{0}")]
    AlreadyExists(String),

    #[error("{0}")]
    NotSupported(String),

    #[error("{0}")]
    InvalidInput(String),

    #[error("{0}")]
    Release(String),

    #[error("{0}")]
    SelfUpdate(String),

    #[error("{0}")]
    Snapshot(String),
}

impl AppError {
    pub fn command_not_available(name: &str) -> Self {
        AppError::CommandNotAvailable(name.to_string())
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        AppError::NotFound(msg.into())
    }

    pub fn already_exists(msg: impl Into<String>) -> Self {
        AppError::AlreadyExists(msg.into())
    }

    pub fn not_supported(msg: impl Into<String>) -> Self {
        AppError::NotSupported(msg.into())
    }

    pub fn invalid_input(msg: impl Into<String>) -> Self {
        AppError::InvalidInput(msg.into())
    }

    pub fn release(msg: impl Into<String>) -> Self {
        AppError::Release(msg.into())
    }

    pub fn self_update(msg: impl Into<String>) -> Self {
        AppError::SelfUpdate(msg.into())
    }

    pub fn snapshot(msg: impl Into<String>) -> Self {
        AppError::Snapshot(msg.into())
    }
}
