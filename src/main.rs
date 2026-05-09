mod cli;
mod commands;
mod domain;
mod utils;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    cli::dispatch(cli)
}
