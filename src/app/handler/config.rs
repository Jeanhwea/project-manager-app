use crate::app::common::config;
use anyhow::Result;
use colored::Colorize;

pub fn execute_init() -> Result<()> {
    let dir = config::config_dir();
    if dir.exists() {
        anyhow::bail!("配置目录已存在: {}", dir.display());
    }

    std::fs::create_dir_all(&dir)?;

    let config_path = config::config_path();
    let content = config::default_config_content();
    std::fs::write(&config_path, content)?;

    let gitlab_path = config::gitlab_config_path();
    let gitlab_content = config::default_gitlab_config_content();
    std::fs::write(&gitlab_path, gitlab_content)?;

    println!("{} {}", "已创建配置目录:".green(), dir.display());
    println!("  {} {}", "主配置:".dimmed(), config_path.display());
    println!("  {} {}", "GitLab:".dimmed(), gitlab_path.display());
    Ok(())
}

pub fn execute_show() -> Result<()> {
    let dir = config::config_dir();
    let cfg = config::load();
    let gitlab_cfg = config::load_gitlab();
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

pub fn execute_path() -> Result<()> {
    println!("{}", config::config_dir().display());
    Ok(())
}
