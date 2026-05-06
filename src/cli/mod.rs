mod args;
mod commands;
mod dispatcher;
mod parser;
mod styles;

pub use args::BumpType;
pub use commands::CommandArgs;
pub use dispatcher::{CommandDispatcher, CommandDispatcherImpl};
pub use parser::{ClapParser, CliParser};
pub use styles::get_styles;

pub type CliResult = Result<(), anyhow::Error>;
