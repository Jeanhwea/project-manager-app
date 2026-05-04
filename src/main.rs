mod cli;
mod commands;
mod domain;
mod utils;

use anyhow::Result;
use cli::{CliParser, CommandDispatcher, CommandDispatcherImpl, ClapParser};

fn main() -> Result<()> {
    let parsed_command = ClapParser::parse()?;
    CommandDispatcherImpl::dispatch(parsed_command)?;
    Ok(())
}
