mod cli;
mod cli_old;
mod commands;
mod domain;
mod utils;

use anyhow::Result;
use cli::{parser::ClapParser, dispatcher::CommandDispatcherImpl, CliParser, CommandDispatcher};

fn main() -> Result<()> {
    // Parse command line arguments using the new CLI parser
    let parsed_command = ClapParser::parse()?;
    
    // Dispatch the command to the appropriate handler
    CommandDispatcherImpl::dispatch(parsed_command)?;
    
    Ok(())
}
