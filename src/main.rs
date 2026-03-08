mod app;
mod cli;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Release { bump_type } => {
            app::release::execute(bump_type.as_str())?;
        }
        Commands::Sync { path, max_depth } => {
            app::sync::execute(&path, max_depth)?;
        }
        Commands::Doctor {
            path,
            max_depth,
            gc,
        } => {
            app::doctor::execute(&path, max_depth, gc)?;
        }
    }

    Ok(())
}
