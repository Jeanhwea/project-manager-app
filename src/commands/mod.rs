//! Commands module
//!
//! Defines command implementations and their argument types.

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

/// Command trait for executing commands
pub trait Command {
    type Args;
    fn execute(args: Self::Args) -> CommandResult;
}

/// Command result type
pub type CommandResult = Result<(), CommandError>;

/// Command error type
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("参数无效: {0}")]
    #[allow(dead_code)]
    InvalidArguments(String),

    #[error("执行失败: {0}")]
    ExecutionFailed(String),

    #[error("Git 错误: {0}")]
    Git(#[from] crate::domain::git::GitError),

    #[error("编辑器错误: {0}")]
    Editor(#[from] crate::domain::editor::EditorError),

    #[error("配置错误: {0}")]
    Config(#[from] crate::domain::config::ConfigError),

    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("验证错误: {0}")]
    Validation(String),
}
