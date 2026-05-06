use super::commands::{Cli, CommandArgs, Commands};
use clap::Parser;

pub trait CliParser {
    fn parse() -> Result<CommandArgs, anyhow::Error>;
}

pub struct ClapParser;

impl CliParser for ClapParser {
    fn parse() -> Result<CommandArgs, anyhow::Error> {
        let cli = Cli::parse();

        let args = match cli.command {
            Commands::Release(args) => CommandArgs::Release(args),
            Commands::Sync(args) => CommandArgs::Sync(args),
            Commands::Doctor(args) => CommandArgs::Doctor(args),
            Commands::Fork(args) => CommandArgs::Fork(args),
            Commands::Gitlab { command } => CommandArgs::GitLab(command),
            Commands::Snap { command } => CommandArgs::Snap(command),
            Commands::Status(args) => CommandArgs::Status(args),
            Commands::Branch { command } => CommandArgs::Branch(command),
            Commands::Self_ { command } => CommandArgs::SelfMan(command),
            Commands::Config { command } => CommandArgs::Config(command),
        };

        Ok(args)
    }
}
