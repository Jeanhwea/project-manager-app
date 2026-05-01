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
///
/// Each command implements this trait with its own argument type.
/// This ensures type safety and clear separation between command domains.
pub trait Command {
    /// Type representing command-specific arguments
    type Args;

    /// Execute the command with the given arguments
    fn execute(args: Self::Args) -> CommandResult;
}

/// Command execution result
pub type CommandResult = Result<(), CommandError>;

/// Command error type
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("Invalid arguments: {0}")]
    #[allow(dead_code)]
    InvalidArguments(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Domain error: {0}")]
    Domain(#[from] crate::domain::DomainError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Validation error: {0}")]
    Validation(String),
}
