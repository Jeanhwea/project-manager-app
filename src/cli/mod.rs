//! CLI infrastructure module
//!
//! This module contains the command-line interface infrastructure including
//! argument parsing, command dispatching, and user interaction.

pub mod cli;
pub mod parser;
pub mod dispatcher;

/// CLI parsing and command execution result
pub type CliResult = Result<(), anyhow::Error>;

/// Command name enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum CommandName {
    Release,
    Sync,
    Doctor,
    Fork,
    GitLab,
    Snap,
    Status,
    Branch,
    SelfMan,
    Config,
}

/// Parsed command structure
#[derive(Debug)]
pub struct ParsedCommand {
    pub name: CommandName,
    pub args: CommandArgs,
}

/// Command arguments enum - can hold different argument types for different commands
#[derive(Debug)]
pub enum CommandArgs {
    Release(crate::commands::release::ReleaseArgs),
    Sync(crate::commands::sync::SyncArgs),
    Doctor(crate::commands::doctor::DoctorArgs),
    Fork(crate::commands::fork::ForkArgs),
    GitLab(crate::commands::gitlab::GitLabArgs),
    Snap(crate::commands::snap::SnapArgs),
    Status(crate::commands::status::StatusArgs),
    Branch(crate::commands::branch::BranchArgs),
    SelfMan(crate::commands::selfman::SelfManArgs),
    Config(crate::commands::config::ConfigArgs),
}

/// CLI parser trait
pub trait CliParser {
    fn parse() -> Result<ParsedCommand, anyhow::Error>;
}

/// Command dispatcher trait  
pub trait CommandDispatcher {
    fn dispatch(command: ParsedCommand) -> CliResult;
}