pub mod command;
pub mod models;
pub mod remote;
pub mod repository;

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("Git command failed: {0}")]
    CommandFailed(String),

    #[error("Invalid remote URL: {0}")]
    #[allow(dead_code)]
    InvalidRemoteUrl(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, GitError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_protocol_invalid() {
        assert!(
            GitError::InvalidRemoteUrl("test".to_string())
                .to_string()
                .contains("test")
        );
    }
}
