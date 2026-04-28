use crate::app::common::config;
use anyhow::Result;
use colored::Colorize;

pub fn execute_init() -> Result<()> {
    let path = config::config_path();
    if path.exists() {
        anyhow::bail!("配置文件已存在: {}", path.display());
    }

    let content = config::default_config_content();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, content)?;

    println!("{} {}", "已创建配置文件:".green(), path.display());
    println!(
        "{} 编辑配置文件以自定义设置: {}",
        "提示:".yellow(),
        path.display()
    );
    Ok(())
}

pub fn execute_show() -> Result<()> {
    let path = config::config_path();
    let cfg = config::load();
    let file_exists = path.exists();

    println!(
        "{} {} {}",
        "配置文件:".green(),
        path.display(),
        if file_exists {
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
        if !rule.path_prefixes.is_empty() {
            if let Some(ref prefix_name) = rule.path_prefix_name {
                println!("    {} <- {:?}", prefix_name.yellow(), rule.path_prefixes);
            }
        }
    }
    println!();

    println!("{}", "[sync]".cyan());
    println!("  skip_push_hosts = {:?}", cfg.sync.skip_push_hosts);

    Ok(())
}

pub fn execute_path() -> Result<()> {
    let path = config::config_path();
    println!("{}", path.display());
    Ok(())
}
