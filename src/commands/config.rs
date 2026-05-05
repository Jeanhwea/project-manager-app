use super::{Command, CommandResult};
use crate::domain::config::ConfigDir;
use crate::domain::config::schema;
use crate::utils::output::Output;

/// Configuration command arguments
#[derive(Debug, clap::Subcommand)]
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

    std::fs::create_dir_all(&dir).map_err(super::CommandError::Io)?;

    let config_path = ConfigDir::config_path();
    std::fs::write(&config_path, schema::default_config_content())
        .map_err(super::CommandError::Io)?;

    let gitlab_path = ConfigDir::gitlab_path();
    std::fs::write(&gitlab_path, schema::default_gitlab_config_content())
        .map_err(super::CommandError::Io)?;

    Output::item("已创建配置目录", &dir.display().to_string());
    Output::detail("主配置", &config_path.display().to_string());
    Output::detail("GitLab", &gitlab_path.display().to_string());
    Ok(())
}

fn execute_show() -> CommandResult {
    let dir = ConfigDir::dir();
    let cfg = ConfigDir::load_config();
    let gitlab_cfg = ConfigDir::load_gitlab();
    let dir_exists = dir.exists();

    let dir_status = if dir_exists {
        "".to_string()
    } else {
        " (未创建, 使用默认值)".to_string()
    };
    Output::item("配置目录", &format!("{}{}", dir.display(), dir_status));

    Output::section("[repository]");
    Output::message(&format!("max_depth  = {}", cfg.repository.max_depth));
    Output::message(&format!("skip_dirs  = {:?}", cfg.repository.skip_dirs));

    Output::section("[remote]");
    for rule in &cfg.remote.rules {
        Output::item(&rule.name, &format!("{:?}", rule.hosts));
        if let Some(ref url_prefix) = rule.url_prefix {
            Output::detail("url_prefix", url_prefix);
        }
        if !rule.path_prefixes.is_empty()
            && let Some(ref prefix_name) = rule.path_prefix_name
        {
            Output::detail(prefix_name, &format!("{:?}", rule.path_prefixes));
        }
    }

    Output::section("[sync]");
    Output::message(&format!("skip_push_hosts = {:?}", cfg.sync.skip_push_hosts));

    Output::section("[gitlab]");
    if gitlab_cfg.servers.is_empty() {
        Output::skip("未配置 GitLab 服务器 (使用 pma gitlab login 添加)");
    } else {
        for srv in &gitlab_cfg.servers {
            Output::detail(&srv.url, &srv.protocol);
        }
    }

    Ok(())
}

fn execute_path() -> CommandResult {
    Output::message(&ConfigDir::dir().display().to_string());
    Ok(())
}
