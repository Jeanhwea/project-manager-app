mod app;
mod cli;
mod utils;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands, SelfCommands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Release {
            bump_type,
            files,
            no_root,
            force,
            skip_push,
            dry_run,
        } => {
            app::handler::release::execute(
                bump_type.as_str(),
                &files,
                no_root,
                force,
                skip_push,
                dry_run,
            )?;
        }
        Commands::Sync {
            path,
            max_depth,
            skip_remotes,
            all_branch,
        } => {
            let skip_remotes = parse_comma_separated(skip_remotes);
            app::handler::sync::execute(&path, max_depth, all_branch, skip_remotes)?;
        }
        Commands::Doctor {
            path,
            max_depth,
            gc,
            rename,
        } => {
            app::handler::doctor::execute(&path, max_depth, gc, rename)?;
        }
        Commands::Fork { path, name } => {
            app::handler::fork::execute(&path, &name)?;
        }
        Commands::Snap { path } => {
            app::handler::snap::execute(&path)?;
        }
        Commands::Self_ { command } => match command {
            SelfCommands::Update { force } => {
                app::handler::selfman::execute(force)?;
            }
            SelfCommands::Version => {
                app::handler::selfman::show_version();
            }
        },
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
