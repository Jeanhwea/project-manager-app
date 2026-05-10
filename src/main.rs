mod cli;
mod commands;
mod control;
mod domain;
mod model;
mod utils;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    cli::dispatch(cli)
}
