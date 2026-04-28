mod app;
mod cli;
mod utils;

use anyhow::Result;
use clap::Parser;
use cli::{BranchCommands, Cli, CloneProtocolType, Commands, ConfigCommands, SelfCommands, SnapCommands};

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
            message,
            pre_release,
        } => {
            app::handler::release::execute(
                bump_type.as_str(),
                &files,
                no_root,
                force,
                skip_push,
                dry_run,
                message.as_deref(),
                pre_release.as_deref(),
            )?;
        }
        Commands::Sync {
            path,
            max_depth,
            skip_remotes,
            all_branch,
            dry_run,
            fetch_only,
            rebase,
        } => {
            let skip_remotes = parse_comma_separated(skip_remotes);
            app::handler::sync::execute(
                &path,
                max_depth,
                all_branch,
                skip_remotes,
                dry_run,
                fetch_only,
                rebase,
            )?;
        }
        Commands::Doctor {
            path,
            max_depth,
            gc,
            rename,
            fix,
            dry_run,
        } => {
            app::handler::doctor::execute(&path, max_depth, gc, rename, fix, dry_run)?;
        }
        Commands::Fork {
            path,
            name,
            dry_run,
        } => {
            app::handler::fork::execute(&path, &name, dry_run)?;
        }
        Commands::Clone {
            group,
            server,
            token,
            protocol,
            output,
            include_archived,
            recursive,
            dry_run,
        } => {
            let clone_protocol = match protocol {
                CloneProtocolType::Ssh => app::handler::clone::CloneProtocol::Ssh,
                CloneProtocolType::Https => app::handler::clone::CloneProtocol::Https,
            };
            app::handler::clone::execute(
                &group,
                &server,
                token.as_deref(),
                &clone_protocol,
                &output,
                include_archived,
                recursive,
                dry_run,
            )?;
        }
        Commands::Snap { command } => match command {
            SnapCommands::Create { path, dry_run } => {
                app::handler::snap::execute(&path, dry_run)?;
            }
            SnapCommands::List { path } => {
                app::handler::snap::execute_list(&path)?;
            }
            SnapCommands::Restore {
                snapshot,
                path,
                dry_run,
            } => {
                app::handler::snap::execute_restore(&path, &snapshot, dry_run)?;
            }
        },
        Commands::Status {
            path,
            max_depth,
            short,
            filter,
        } => {
            let filter = filter.map(|f| {
                app::handler::status::StatusFilter::from_str(f.as_str())
                    .expect("invalid filter value")
            });
            app::handler::status::execute(&path, max_depth, short, filter)?;
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
            BranchCommands::Switch {
                branch,
                create,
                path,
                max_depth,
                dry_run,
            } => {
                app::handler::branch::execute_switch(&path, max_depth, &branch, create, dry_run)?;
            }
            BranchCommands::Rename {
                old_name,
                new_name,
                path,
                max_depth,
                dry_run,
            } => {
                app::handler::branch::execute_rename(
                    &path, max_depth, &old_name, &new_name, dry_run,
                )?;
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
