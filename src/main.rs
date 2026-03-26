mod app;
mod cli;
mod utils;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Release {
            bump_type,
            no_root,
            force,
        } => {
            app::release::execute(bump_type.as_str(), no_root, force)?;
        }
        Commands::Sync {
            path,
            max_depth,
            skip_remotes,
            all_branch,
        } => {
            let skip_remotes = parse_comma_separated(skip_remotes);
            app::sync::execute(&path, max_depth, all_branch, skip_remotes)?;
        }
        Commands::Doctor {
            path,
            max_depth,
            gc,
        } => {
            app::doctor::execute(&path, max_depth, gc)?;
        }
        Commands::Fork { path, name } => {
            app::fork::execute(&path, &name)?;
        }
        Commands::Snap { path } => {
            app::snap::execute(&path)?;
        }
    }

    Ok(())
}

/// 处理逗号分隔的参数值，将 "a,b,c" 展开为 ["a", "b", "c"]
fn parse_comma_separated(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .flat_map(|v| {
            v.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        })
        .collect()
}
