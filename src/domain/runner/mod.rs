//! Runner domain module
//!
//! This module contains command execution abstractions.

/// Runner-specific error type
#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    #[error("Command execution failed: {0}")]
    CommandFailed(String),
    
    #[error("Command not found: {0}")]
    CommandNotFound(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Timeout: {0}")]
    Timeout(String),
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Command execution context
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub dry_run: bool,
    pub verbose: bool,
    pub timeout: Option<std::time::Duration>,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            dry_run: false,
            verbose: false,
            timeout: None,
        }
    }
}

/// Command runner trait
pub trait CommandRunner {
    fn run(&self, command: &str, args: &[&str], context: &ExecutionContext) -> Result<CommandOutput>;
}

/// Command execution output
#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub status: i32,
    pub duration: std::time::Duration,
}

impl Default for CommandOutput {
    fn default() -> Self {
        Self {
            stdout: String::new(),
            stderr: String::new(),
            status: 0,
            duration: std::time::Duration::from_secs(0),
        }
    }
}

/// Common result type for runner operations
pub type Result<T> = std::result::Result<T, RunnerError>;