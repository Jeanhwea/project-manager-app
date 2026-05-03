mod args;
mod styles;

pub mod dispatcher;
pub mod parser;

pub use args::{
    BranchCommands, BumpType, Commands, ConfigCommands, GitlabCommands, SelfCommands,
    SnapCommands,
};
pub use styles::get_styles;

use clap::Parser;

#[derive(Parser)]
#[command(name = "pma")]
#[command(about = "Project Manager Application (项目管理工具)")]
#[command(version)]
#[command(styles = get_styles())]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

pub type CliResult = Result<(), anyhow::Error>;

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

#[derive(Debug)]
pub struct ParsedCommand {
    pub name: CommandName,
    pub args: CommandArgs,
}

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

pub trait CliParser {
    fn parse() -> Result<ParsedCommand, anyhow::Error>;
}

pub trait CommandDispatcher {
    fn dispatch(command: ParsedCommand) -> CliResult;
}
