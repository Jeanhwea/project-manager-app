//! Command implementations module
//!
//! This module contains the implementation of individual CLI commands
//! following the Command trait pattern.

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

/// Command trait for all command implementations
pub trait Command {
    /// Execute the command with the given arguments
    fn execute(args: CommandArgs) -> CommandResult;
}

/// Command-specific arguments (to be refined per command)
#[derive(Debug)]
pub struct CommandArgs {
    pub raw_args: Vec<String>,
}

/// Command execution result
pub type CommandResult = Result<(), CommandError>;

/// Command error type
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),
    
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Domain error: {0}")]
    Domain(#[from] crate::domain::DomainError),
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Helper to convert domain errors to command errors
impl From<crate::domain::DomainError> for CommandError {
    fn from(error: crate::domain::DomainError) -> Self {
        CommandError::Domain(error)
    }
}