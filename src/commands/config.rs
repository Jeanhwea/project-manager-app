//! Configuration command implementation

use super::{Command, CommandResult};
use crate::domain::config::ConfigDir;
use crate::domain::config::schema;
use colored::Colorize;

/// Configuration command arguments
#[derive(Debug)]
pub enum ConfigArgs {
    /// Initialize a default configuration file
    Init,
    /// Show current configuration
    Show,
    /// Show configuration file path
    Path,
}

/// Config command
pub struct ConfigCommand;

impl Command for ConfigCommand {
    type Args = ConfigArgs;

    fn execute(args: Self::Args) -> CommandResult {
        match args {
            ConfigArgs::Init => execute_init(),
            ConfigArgs::Show => execute_show(),
            ConfigArgs::Path => execute_path(),
        }
    }
}

fn execute_init() -> CommandResult {
    let dir = ConfigDir::dir();
    if dir.exists() {
        return Err(super::CommandError::ExecutionFailed(format!(
            "配置目录已存在: {}",
            dir.display()
        )));
    }

    std::fs::create_dir_all(&dir).map_err(|e| super::CommandError::Io(e))?;

    let config_path = ConfigDir::config_path();
    std::fs::write(&config_path, schema::default_config_content())
        .map_err(|e| super::CommandError::Io(e))?;

    let gitlab_path = ConfigDir::gitlab_path();
    std::fs::write(&gitlab_path, schema::default_gitlab_config_content())
        .map_err(|e| super::CommandError::Io(e))?;

    println!("{} {}", "已创建配置目录:".green(), dir.display());
    println!("  {} {}", "主配置:".dimmed(), config_path.display());
    println!("  {} {}", "GitLab:".dimmed(), gitlab_path.display());
    Ok(())
}

fn execute_show() -> CommandResult {
    let dir = ConfigDir::dir();
    let cfg = ConfigDir::load_config();
    let gitlab_cfg = ConfigDir::load_gitlab();
    let dir_exists = dir.exists();

    println!(
        "{} {} {}",
        "配置目录:".green(),
        dir.display(),
        if dir_exists {
            "".to_string()
        } else {
            "(未创建, 使用默认值)".yellow().to_string()
        }
    );
    println!();

    println!("{}", "[repository]".cyan());
    println!("  max_depth  = {}", cfg.repository.max_depth);
    println!("  skip_dirs  = {:?}", cfg.repository.skip_dirs);
    println!();

    println!("{}", "[remote]".cyan());
    for rule in &cfg.remote.rules {
        println!("  {} <- {:?}", rule.name.yellow(), rule.hosts);
        if let Some(ref url_prefix) = rule.url_prefix {
            println!("    {} = {}", "url_prefix".dimmed(), url_prefix.dimmed());
        }
        if !rule.path_prefixes.is_empty()
            && let Some(ref prefix_name) = rule.path_prefix_name
        {
            println!("    {} <- {:?}", prefix_name.yellow(), rule.path_prefixes);
        }
    }
    println!();

    println!("{}", "[sync]".cyan());
    println!("  skip_push_hosts = {:?}", cfg.sync.skip_push_hosts);
    println!();

    println!("{}", "[gitlab]".cyan());
    if gitlab_cfg.servers.is_empty() {
        println!(
            "  {}",
            "未配置 GitLab 服务器 (使用 pma gitlab login 添加)".dimmed()
        );
    } else {
        for srv in &gitlab_cfg.servers {
            println!("  {} ({})", srv.url.cyan(), srv.protocol.dimmed());
        }
    }

    Ok(())
}

fn execute_path() -> CommandResult {
    println!("{}", ConfigDir::dir().display());
    Ok(())
}
