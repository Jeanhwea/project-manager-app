mod cli;
mod commands;
mod domain;
mod utils;

// Note: The old `app` module has been removed.
// All command logic now lives in `commands/` backed by `domain/` infrastructure.

use anyhow::Result;
use cli::{CliParser, CommandDispatcher, dispatcher::CommandDispatcherImpl, parser::ClapParser};

fn main() -> Result<()> {
    // Parse command line arguments using the new CLI parser
    let parsed_command = ClapParser::parse()?;

    // Dispatch the command to the appropriate handler
    CommandDispatcherImpl::dispatch(parsed_command)?;

    Ok(())
}
