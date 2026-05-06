mod cli;
mod commands;
mod domain;
mod error;
mod utils;

use anyhow::Result;
use cli::{ClapParser, CliParser, CommandDispatcher, CommandDispatcherImpl};

fn main() -> Result<()> {
    let parsed_command = ClapParser::parse()?;
    CommandDispatcherImpl::dispatch(parsed_command)?;
    Ok(())
}
