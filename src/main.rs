mod app;
mod cli;
mod utils;

use anyhow::Result;
use clap::Parser;
use cli::{BranchCommands, Cli, Commands, ConfigCommands, SelfCommands};

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
            dry_run,
        } => {
            let skip_remotes = parse_comma_separated(skip_remotes);
            app::handler::sync::execute(&path, max_depth, all_branch, skip_remotes, dry_run)?;
        }
        Commands::Doctor {
            path,
            max_depth,
            gc,
            rename,
            dry_run,
        } => {
            app::handler::doctor::execute(&path, max_depth, gc, rename, dry_run)?;
        }
        Commands::Fork {
            path,
            name,
            dry_run,
        } => {
            app::handler::fork::execute(&path, &name, dry_run)?;
        }
        Commands::Snap { path, dry_run } => {
            app::handler::snap::execute(&path, dry_run)?;
        }
        Commands::Status {
            path,
            max_depth,
            short,
        } => {
            app::handler::status::execute(&path, max_depth, short)?;
        }
        Commands::Branch { command } => match command {
            BranchCommands::List { path, max_depth } => {
                app::handler::branch::execute_list(&path, max_depth)?;
            }
            BranchCommands::Clean {
                path,
                max_depth,
                remote,
                dry_run,
            } => {
                app::handler::branch::execute_clean(&path, max_depth, remote, dry_run)?;
            }
        },
        Commands::Self_ { command } => match command {
            SelfCommands::Update { force } => {
                app::handler::selfman::execute(force)?;
            }
            SelfCommands::Version => {
                app::handler::selfman::show_version();
            }
        },
        Commands::Config { command } => match command {
            ConfigCommands::Init => {
                app::handler::config::execute_init()?;
            }
            ConfigCommands::Show => {
                app::handler::config::execute_show()?;
            }
            ConfigCommands::Path => {
                app::handler::config::execute_path()?;
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
