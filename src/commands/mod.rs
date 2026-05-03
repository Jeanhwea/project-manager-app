pub mod branch;
pub mod config;
pub mod doctor;
pub mod fork;
pub mod gitlab;
pub mod release;
pub mod selfman;
pub mod snap;
pub mod status;
pub mod sync;

pub trait Command {
    type Args;
    fn execute(args: Self::Args) -> CommandResult;
}

pub type CommandResult = Result<(), CommandError>;

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("Invalid arguments: {0}")]
    #[allow(dead_code)]
    InvalidArguments(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Git error: {0}")]
    Git(#[from] crate::domain::git::GitError),

    #[error("Editor error: {0}")]
    Editor(#[from] crate::domain::editor::EditorError),

    #[error("Configuration error: {0}")]
    Config(#[from] crate::domain::config::ConfigError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Validation error: {0}")]
    Validation(String),
}
