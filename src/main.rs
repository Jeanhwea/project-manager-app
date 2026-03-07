mod app;
mod cli;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Release { bump_type } => {
            app::release::execute(bump_type.as_str());
        }
        Commands::Synchronize { path, max_depth } => {
            app::sync::execute(&path, max_depth);
        }
        Commands::Housekeeping {
            path,
            max_depth,
            gc,
        } => {
            app::housekeeping::execute(&path, max_depth, gc);
        }
    }
}
