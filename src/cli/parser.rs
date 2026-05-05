//! CLI parser implementation

use super::commands::{Cli, CommandArgs, CommandName, Commands, ParsedCommand};
use clap::Parser;

/// Trait for parsing CLI arguments into commands
pub trait CliParser {
    fn parse() -> Result<ParsedCommand, anyhow::Error>;
}

pub struct ClapParser;

impl CliParser for ClapParser {
    fn parse() -> Result<ParsedCommand, anyhow::Error> {
        let cli = Cli::parse();

        let parsed_command = match cli.command {
            Commands::Release(args) => ParsedCommand {
                name: CommandName::Release,
                args: CommandArgs::Release(args),
            },
            Commands::Sync(args) => ParsedCommand {
                name: CommandName::Sync,
                args: CommandArgs::Sync(args),
            },
            Commands::Doctor(args) => ParsedCommand {
                name: CommandName::Doctor,
                args: CommandArgs::Doctor(args),
            },
            Commands::Fork(args) => ParsedCommand {
                name: CommandName::Fork,
                args: CommandArgs::Fork(args),
            },
            Commands::Gitlab { command } => ParsedCommand {
                name: CommandName::GitLab,
                args: CommandArgs::GitLab(command),
            },
            Commands::Snap { command } => ParsedCommand {
                name: CommandName::Snap,
                args: CommandArgs::Snap(command),
            },
            Commands::Status(args) => ParsedCommand {
                name: CommandName::Status,
                args: CommandArgs::Status(args),
            },
            Commands::Branch { command } => ParsedCommand {
                name: CommandName::Branch,
                args: CommandArgs::Branch(command),
            },
            Commands::Self_ { command } => ParsedCommand {
                name: CommandName::SelfMan,
                args: CommandArgs::SelfMan(command),
            },
            Commands::Config { command } => ParsedCommand {
                name: CommandName::Config,
                args: CommandArgs::Config(command),
            },
        };

        Ok(parsed_command)
    }
}
