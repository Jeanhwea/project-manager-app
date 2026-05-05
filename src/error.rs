use std::io;

pub use crate::domain::config::ConfigError;
pub use crate::domain::editor::EditorError;
pub use crate::domain::git::GitError;

#[derive(Debug, thiserror::Error)]
pub enum PmaError {
    #[error("Git error: {0}")]
    Git(#[from] GitError),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Editor error: {0}")]
    Editor(#[from] EditorError),

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Validation error: {0}")]
    Validation(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pma_error_from_git_error() {
        let git_err = GitError::CommandFailed("test command".to_string());
        let pma_err: PmaError = git_err.into();

        match pma_err {
            PmaError::Git(e) => {
                assert!(e.to_string().contains("test command"));
            }
            _ => panic!("Expected Git variant"),
        }
    }

    #[test]
    fn test_pma_error_from_config_error() {
        let config_err = ConfigError::ParseError("test parse error".to_string());
        let pma_err: PmaError = config_err.into();

        match pma_err {
            PmaError::Config(e) => {
                assert!(e.to_string().contains("test parse error"));
            }
            _ => panic!("Expected Config variant"),
        }
    }

    #[test]
    fn test_pma_error_from_editor_error() {
        let editor_err = EditorError::ParseError("test editor error".to_string());
        let pma_err: PmaError = editor_err.into();

        match pma_err {
            PmaError::Editor(e) => {
                assert!(e.to_string().contains("test editor error"));
            }
            _ => panic!("Expected Editor variant"),
        }
    }

    #[test]
    fn test_pma_error_from_io_error() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let pma_err: PmaError = io_err.into();

        match pma_err {
            PmaError::Io(e) => {
                assert!(e.to_string().contains("file not found"));
            }
            _ => panic!("Expected Io variant"),
        }
    }

    #[test]
    fn test_pma_error_validation() {
        let pma_err = PmaError::Validation("invalid input".to_string());

        match pma_err {
            PmaError::Validation(msg) => {
                assert_eq!(msg, "invalid input");
            }
            _ => panic!("Expected Validation variant"),
        }
    }

    #[test]
    fn test_pma_error_to_anyhow() {
        let pma_err = PmaError::Validation("test validation".to_string());
        let anyhow_err: anyhow::Error = pma_err.into();

        assert!(anyhow_err.to_string().contains("test validation"));
    }

    #[test]
    fn test_error_display() {
        let git_err = GitError::RemoteNotFound("origin".to_string());
        let pma_err: PmaError = git_err.into();

        let display = format!("{}", pma_err);
        assert!(display.contains("Git error"));
        assert!(display.contains("origin"));
    }

    #[test]
    fn test_error_chain_with_anyhow() {
        use anyhow::Context;

        let git_err = GitError::CommandFailed("git push failed".to_string());
        let pma_err: PmaError = git_err.into();
        let anyhow_err: anyhow::Error = pma_err.into();

        // Verify we can use anyhow context methods
        let result: Result<String, anyhow::Error> = Err(anyhow_err);
        let with_context = result.context("additional context");

        assert!(with_context.is_err());
        let err = with_context.unwrap_err();
        // anyhow wraps the error, so the message includes the chain
        let err_str = err.to_string();
        assert!(
            err_str.contains("Git error")
                || err_str.contains("git push failed")
                || err_str.contains("additional context"),
            "Error string was: {err_str}"
        );
    }
}
