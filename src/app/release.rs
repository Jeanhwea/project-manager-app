use super::git;
use super::utils;
use super::version::Version;
use anyhow::{Context, Result};
use regex::Regex;
use std::path::Path;

const CARGO_TOML: &str = "Cargo.toml";
const POM_XML: &str = "pom.xml";
const PYPROJECT_TOML: &str = "pyproject.toml";
const PYTHON_VERSION_FILE: &str = "src/__version__.py";
const VERSION_FILE: &str = "version";
const VERSION_TEXT: &str = "version.txt";

pub fn execute(bump_type: &str) -> Result<()> {
    let current_branch = git::get_current_branch().unwrap_or_else(|| "master".to_string());
    if current_branch != "master" {
        anyhow::bail!("只能在 master 分支上执行 release");
    }

    let current_tag = git::get_current_version().unwrap_or_else(|| "v0.0.0".to_string());

    let rev_current_tag = git::get_rev_revision(&current_tag)?;
    let rev_head = git::get_rev_revision("HEAD")?;
    if rev_current_tag == rev_head {
        anyhow::bail!("当前 HEAD 已被标记为 {}", current_tag);
    }

    let version = Version::from_tag(&current_tag).unwrap_or_default();
    let new_version = version.bump(bump_type);
    let new_tag = new_version.to_tag();

    let config_files = detect_config_file()?;
    for config_file in config_files {
        release_config_file(&new_tag, &config_file)?;
        git::add_file(&config_file)?;
    }

    git::list_cached_changes()?;
    git::commit(&new_tag.to_string())?;
    git::create_tag(&new_tag)?;

    if let Some(remotes) = git::get_remote_list() {
        for remote in remotes {
            if let Err(e) = git::push_tag(&remote, &new_tag) {
                eprintln!("警告: 推送标签失败: {}", e);
            }
            if let Err(e) = git::push_branch(&remote, &current_branch) {
                eprintln!("警告: 推送分支失败: {}", e);
            }
        }
    }

    Ok(())
}

pub fn release_config_file(tag: &str, config_file: &str) -> Result<()> {
    let dir_name = utils::get_current_dir()?;
    let python_project_version_file = format!("{}/__version__.py", dir_name);

    if config_file == CARGO_TOML {
        edit_cargo_toml_file(tag, config_file)?;
    } else if config_file == POM_XML {
        edit_pom_xml_file(tag, config_file)?;
    } else if config_file == PYPROJECT_TOML {
        edit_pyproject_toml_file(tag, config_file)?;
    } else if config_file == PYTHON_VERSION_FILE || config_file == python_project_version_file {
        edit_python_package_init_file(tag, config_file)?;
    } else if config_file == VERSION_FILE {
        edit_version_text_file(tag, config_file)?;
    } else if config_file == VERSION_TEXT {
        edit_version_text_file(tag, config_file)?;
    } else {
        anyhow::bail!("不支持的配置文件 {}", config_file);
    }
    Ok(())
}

fn detect_config_file() -> Result<Vec<String>> {
    let mut config_files = Vec::new();

    if Path::new(CARGO_TOML).exists() {
        config_files.push(CARGO_TOML.to_string());
    }
    if Path::new(POM_XML).exists() {
        config_files.push(POM_XML.to_string());
    }
    if Path::new(PYPROJECT_TOML).exists() {
        config_files.push(PYPROJECT_TOML.to_string());
    }
    if Path::new(PYTHON_VERSION_FILE).exists() {
        config_files.push(PYTHON_VERSION_FILE.to_string());
    }

    if Path::new(VERSION_FILE).exists() {
        config_files.push(VERSION_FILE.to_string());
    }
    if Path::new(VERSION_TEXT).exists() {
        config_files.push(VERSION_TEXT.to_string());
    }

    let dir_name = utils::get_current_dir()?;
    let python_project_version_file = format!("{}/__version__.py", dir_name);
    if Path::new(&python_project_version_file).exists() {
        config_files.push(python_project_version_file);
    }

    if config_files.is_empty() {
        anyhow::bail!("未检测到可编辑的配置文件");
    }

    Ok(config_files)
}

fn edit_cargo_toml_file(tag: &str, config_file: &str) -> Result<()> {
    let config_content = std::fs::read_to_string(config_file)
        .with_context(|| format!("无法读取 {}", config_file))?;

    let version = tag.trim_start_matches('v');

    let mut lines: Vec<String> = config_content.lines().map(|s| s.to_string()).collect();
    let mut in_package_section = false;

    for line in &mut lines {
        if line.trim() == "[package]" {
            in_package_section = true;
        } else if line.starts_with('[') && !line.trim().is_empty() {
            in_package_section = false;
        }

        if in_package_section && line.trim().starts_with("version = ") {
            *line = format!("version = \"{}\"", version);
            break;
        }
    }

    let new_config_content = lines.join("\n");

    std::fs::write(config_file, new_config_content)
        .with_context(|| format!("无法写入 {}", config_file))?;

    Ok(())
}

fn edit_pom_xml_file(tag: &str, config_file: &str) -> Result<()> {
    let config_content = std::fs::read_to_string(config_file)
        .with_context(|| format!("无法读取 {}", config_file))?;

    let re = Regex::new(r#"<version>[0-9]+\.[0-9]+\.[0-9]+</version>"#).unwrap();
    let new_config_content =
        re.replace(&config_content, &format!(r#"<version>{}</version>"#, tag));

    std::fs::write(config_file, new_config_content.to_string())
        .with_context(|| format!("无法写入 {}", config_file))?;

    Ok(())
}

fn edit_pyproject_toml_file(tag: &str, config_file: &str) -> Result<()> {
    let config_content = std::fs::read_to_string(config_file)
        .with_context(|| format!("无法读取 {}", config_file))?;

    let version = tag.trim_start_matches('v');

    let mut lines: Vec<String> = config_content.lines().map(|s| s.to_string()).collect();
    let mut in_project_section = false;

    for line in &mut lines {
        if line.trim() == "[project]" {
            in_project_section = true;
        } else if line.starts_with('[') && !line.trim().is_empty() {
            in_project_section = false;
        }

        if in_project_section && line.trim().starts_with("version = ") {
            *line = format!("version = \"{}\"", version);
            break;
        }
    }

    let new_config_content = lines.join("\n");

    std::fs::write(config_file, new_config_content)
        .with_context(|| format!("无法写入 {}", config_file))?;

    Ok(())
}

fn edit_python_package_init_file(tag: &str, config_file: &str) -> Result<()> {
    let version = tag.trim_start_matches('v');

    let content = std::fs::read_to_string(config_file)
        .with_context(|| format!("无法读取 {}", config_file))?;

    let re = Regex::new(r#"__version__ = "[^"]*""#).unwrap();
    let new_content = re.replace(&content, &format!(r#"__version__ = "{}""#, version));

    std::fs::write(config_file, new_content.as_ref())
        .with_context(|| format!("无法写入 {}", config_file))?;

    Ok(())
}

fn edit_version_text_file(tag: &str, config_file: &str) -> Result<()> {
    let version = tag.trim_start_matches('v');

    std::fs::write(config_file, version).with_context(|| format!("无法写入 {}", config_file))?;

    Ok(())
}
