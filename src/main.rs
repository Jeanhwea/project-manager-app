mod cli;
mod commands;
mod control;
mod domain;
mod error;
mod model;
mod utils;

use crate::error::AppError;
use clap::Parser;

fn main() -> Result<(), AppError> {
    let cli = cli::Cli::parse();
    cli::dispatch(cli)
}
