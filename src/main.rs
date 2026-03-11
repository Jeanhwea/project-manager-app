mod app;
mod cli;
mod utils;

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
            all_branch,
        } => {
            // 处理逗号分隔的远程仓库名称
            let mut processed_remotes = Vec::new();
            for remote in skip_remotes {
                if remote.contains(',') {
                    // 分割逗号分隔的字符串
                    let split_remotes: Vec<String> = remote
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    processed_remotes.extend(split_remotes);
                } else {
                    processed_remotes.push(remote);
                }
            }
            app::sync::execute(&path, max_depth, all_branch, processed_remotes)?;
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
    }

    Ok(())
}
