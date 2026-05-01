#[allow(clippy::module_inception)]
pub mod cli;
pub mod dispatcher;
pub mod parser;

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
