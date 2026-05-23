mod cli;
mod commands;
mod domain;
mod engine;
mod error;
mod model;
mod utils;

use crate::error::AppError;
use clap::Parser;
use std::error::Error;
use std::io::Write;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = cli::Cli::parse();
    match cli::dispatch(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            let mut stderr = std::io::stderr();
            render_error_chain(&err, &mut stderr);
            ExitCode::FAILURE
        }
    }
}

fn render_error_chain<W: Write>(err: &AppError, w: &mut W) {
    let _ = writeln!(w, "error: {}", err);
    let mut current: Option<&dyn Error> = err.source();
    while let Some(cause) = current {
        let _ = writeln!(w, "caused by: {}", cause);
        current = cause.source();
    }
}
