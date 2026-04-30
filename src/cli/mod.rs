//! CLI infrastructure module
//!
//! This module contains the command-line interface infrastructure including
//! argument parsing, command dispatching, and user interaction.

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

/// Command arguments (to be refined per command)
#[derive(Debug)]
pub struct CommandArgs {
    pub raw_args: Vec<String>,
}

/// CLI parser trait
pub trait CliParser {
    fn parse() -> Result<ParsedCommand, anyhow::Error>;
}

/// Command dispatcher trait  
pub trait CommandDispatcher {
    fn dispatch(command: ParsedCommand) -> CliResult;
}