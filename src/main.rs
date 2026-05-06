mod cli;
mod commands;
mod domain;
mod error;
mod utils;

use anyhow::Result;
use cli::{ClapParser, CliParser, CommandDispatcher, CommandDispatcherImpl};

fn main() -> Result<()> {
    let args = ClapParser::parse()?;
    CommandDispatcherImpl::dispatch(args)?;
    Ok(())
}
