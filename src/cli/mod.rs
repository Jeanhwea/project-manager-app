//! CLI command definitions
//!
//! This module re-exports command types from the commands module for CLI parsing.
//! The Args structures derive clap::Args/Subcommand/ValueEnum for CLI parsing.

mod args;
mod commands;
mod dispatcher;
mod parser;
mod styles;

pub use args::BumpType;
pub use commands::{CommandArgs, CommandName, ParsedCommand};
pub use dispatcher::{CommandDispatcher, CommandDispatcherImpl};
pub use parser::{ClapParser, CliParser};
pub use styles::get_styles;

pub type CliResult = Result<(), anyhow::Error>;
