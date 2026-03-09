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
        Commands::Sync {
            path,
            max_depth,
            skip_remotes,
        } => {
            app::sync::execute(&path, max_depth, skip_remotes)?;
        }
        Commands::Doctor {
            path,
            max_depth,
            gc,
        } => {
            app::doctor::execute(&path, max_depth, gc)?;
        }
        Commands::Init { path, name } => {
            app::init::execute(&path, &name)?;
        }
    }

    Ok(())
}
