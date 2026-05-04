mod args;
mod commands;
mod dispatcher;
mod parser;
mod styles;

pub use args::BumpType;
pub use commands::{
    BranchCommands, Cli, CommandArgs, CommandName, Commands, ConfigCommands, GitlabCommands,
    ParsedCommand, SelfCommands, SnapCommands,
};
pub use dispatcher::{CommandDispatcher, CommandDispatcherImpl};
pub use parser::{CliParser, ClapParser};
pub use styles::get_styles;

pub type CliResult = Result<(), anyhow::Error>;
