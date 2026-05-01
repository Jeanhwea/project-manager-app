mod cli;
mod commands;
mod domain;
mod utils;

use anyhow::Result;
use cli::{CliParser, CommandDispatcher, dispatcher::CommandDispatcherImpl, parser::ClapParser};

fn main() -> Result<()> {
    // Parse command line arguments using the new CLI parser
    let parsed_command = ClapParser::parse()?;

    // Dispatch the command to the appropriate handler
    CommandDispatcherImpl::dispatch(parsed_command)?;

    Ok(())
}
