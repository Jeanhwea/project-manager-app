use crate::domain::AppError;
use crate::domain::config::ConfigDir;
use crate::domain::config::schema;
use crate::utils::output::Output;

#[derive(Debug, clap::Subcommand)]
pub enum ConfigArgs {
    /// Initialize a default configuration file
    Init,
    /// Show current configuration
    Show,
    /// Show configuration file path
    Path,
}

pub fn run(args: ConfigArgs) -> anyhow::Result<()> {
    match args {
        ConfigArgs::Init => execute_init(),
        ConfigArgs::Show => execute_show(),
        ConfigArgs::Path => execute_path(),
    }
}

fn execute_init() -> anyhow::Result<()> {
    let dir = ConfigDir::base_dir();
    if dir.exists() {
        return Err(
            AppError::already_exists(format!("配置目录已存在: {}", dir.display())).into(),
        );
    }

    std::fs::create_dir_all(&dir)?;
    std::fs::write(ConfigDir::config_path(), schema::default_config_content())?;
    std::fs::write(
        ConfigDir::gitlab_path(),
        schema::default_gitlab_config_content(),
    )?;

    Output::item("已创建配置目录", &dir.display().to_string());
    Output::detail("主配置", &ConfigDir::config_path().display().to_string());
    Output::detail("GitLab", &ConfigDir::gitlab_path().display().to_string());
    Ok(())
}

fn execute_show() -> anyhow::Result<()> {
    let dir = ConfigDir::base_dir();
    let cfg = ConfigDir::load_config();
    let gitlab_cfg = ConfigDir::load_gitlab();

    let dir_status = if dir.exists() {
        String::new()
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

fn execute_path() -> anyhow::Result<()> {
    Output::message(&ConfigDir::base_dir().display().to_string());
    Ok(())
}
