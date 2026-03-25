use super::git;
use super::version::Version;
use anyhow::{Context, Result};
use regex::Regex;
use std::path::Path;

/// 配置文件类型及其对应的候选路径
const CONFIG_FILE_CANDIDATES: &[(&[&str], ConfigFileType)] = &[
    (
        &["Cargo.toml", "src-tauri/Cargo.toml"],
        ConfigFileType::CargoToml,
    ),
    (&["pom.xml"], ConfigFileType::PomXml),
    (&["pyproject.toml"], ConfigFileType::PyprojectToml),
    (&["{}/__version__.py"], ConfigFileType::PythonVersion),
    (&["version", "version.txt"], ConfigFileType::VersionText),
    (
        &["package.json", "apps/{}/package.json", "ui/package.json"],
        ConfigFileType::PackageJson,
    ),
];

#[derive(Clone, Copy)]
enum ConfigFileType {
    CargoToml,
    PomXml,
    PyprojectToml,
    PythonVersion,
    VersionText,
    PackageJson,
}

pub fn execute(bump_type: &str, force_current_dir: bool) -> Result<()> {
    // 除非指定 --force-current-dir，否则先切换到 git 仓库根目录
    if !force_current_dir
        && let Some(root) = git::get_top_level_dir()
        && let Err(e) = std::env::set_current_dir(&root)
    {
        eprintln!("警告: 无法切换到 git 根目录: {}, {}", root.display(), e);
    }

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

    let config_files = detect_config_files()?;
    for (file_path, file_type) in &config_files {
        edit_version_in_file(&new_tag, file_path, *file_type)?;
        git::add_file(file_path)?;
    }

    git::list_cached_changes()?;
    git::commit(&new_tag)?;
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

fn detect_config_files() -> Result<Vec<(String, ConfigFileType)>> {
    let mut result = Vec::new();

    for (candidates, file_type) in CONFIG_FILE_CANDIDATES {
        for pattern in *candidates {
            if pattern.contains("{}") {
                // 动态搜索: 将 {} 替换为当前目录下匹配的子目录
                for path in expand_glob_pattern(pattern) {
                    if Path::new(&path).exists() {
                        result.push((path, *file_type));
                    }
                }
            } else if Path::new(pattern).exists() {
                result.push((pattern.to_string(), *file_type));
            }
        }
    }

    if result.is_empty() {
        anyhow::bail!("未检测到可编辑的配置文件");
    }

    Ok(result)
}

/// 展开含 `{}` 占位符的路径模式，搜索匹配的子目录
fn expand_glob_pattern(pattern: &str) -> Vec<String> {
    let mut results = Vec::new();
    let (prefix, suffix) = match pattern.split_once("{}") {
        Some(pair) => pair,
        None => return results,
    };

    // 确定要扫描的目录（prefix 为空则扫描当前目录）
    let scan_dir = if prefix.is_empty() {
        ".".to_string()
    } else {
        prefix.trim_end_matches('/').to_string()
    };

    let entries = match std::fs::read_dir(&scan_dir) {
        Ok(e) => e,
        Err(_) => return results,
    };

    for entry in entries.flatten() {
        if entry.path().is_dir() {
            let dir_name = entry.file_name().to_string_lossy().to_string();
            // 跳过隐藏目录和 node_modules
            if dir_name.starts_with('.') || dir_name == "node_modules" {
                continue;
            }
            let candidate = format!("{}{}{}", prefix, dir_name, suffix);
            results.push(candidate);
        }
    }

    results
}

fn edit_version_in_file(tag: &str, config_file: &str, file_type: ConfigFileType) -> Result<()> {
    match file_type {
        ConfigFileType::VersionText => {
            let version = tag.trim_start_matches('v');
            std::fs::write(config_file, version)
                .with_context(|| format!("无法写入 {}", config_file))?;
            return Ok(());
        }
        ConfigFileType::PythonVersion => {
            return edit_with_regex(config_file, tag, r#"__version__ = "[^"]*""#, |v| {
                format!(r#"__version__ = "{}""#, v)
            });
        }
        _ => {}
    }

    // 基于行的版本替换（Cargo.toml, pom.xml, pyproject.toml, package.json）
    let content = std::fs::read_to_string(config_file)
        .with_context(|| format!("无法读取 {}", config_file))?;
    let version = tag.trim_start_matches('v');
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    match file_type {
        ConfigFileType::CargoToml => {
            replace_in_section(&mut lines, "[package]", "version = ", || {
                format!("version = \"{}\"", version)
            });
        }
        ConfigFileType::PyprojectToml => {
            replace_in_section(&mut lines, "[project]", "version = ", || {
                format!("version = \"{}\"", version)
            });
        }
        ConfigFileType::PomXml => {
            let re = Regex::new(r#"<version>[^<]*</version>"#)?;
            for line in &mut lines {
                if line.trim().starts_with("<version>") {
                    *line = re
                        .replace(line, &format!(r#"<version>{}</version>"#, version))
                        .to_string();
                    break;
                }
            }
        }
        ConfigFileType::PackageJson => {
            let re = Regex::new(r#""version":\s*"[^"]*""#)?;
            for line in &mut lines {
                if line.trim().starts_with("\"version\": ") {
                    *line = re
                        .replace(line, &format!(r#""version": "{}""#, version))
                        .to_string();
                    break;
                }
            }
        }
        _ => unreachable!(),
    }

    write_lines(config_file, &lines, content.ends_with('\n'))
}

/// 在 TOML section 内替换匹配行
fn replace_in_section<F>(lines: &mut [String], section: &str, prefix: &str, replacement: F)
where
    F: FnOnce() -> String,
{
    let mut in_section = false;
    for line in lines.iter_mut() {
        if line.trim() == section {
            in_section = true;
        } else if line.starts_with('[') && !line.trim().is_empty() {
            in_section = false;
        }
        if in_section && line.trim().starts_with(prefix) {
            *line = replacement();
            break;
        }
    }
}

/// 使用正则表达式替换整个文件内容
fn edit_with_regex(
    config_file: &str,
    tag: &str,
    pattern: &str,
    replacement_fn: impl FnOnce(&str) -> String,
) -> Result<()> {
    let version = tag.trim_start_matches('v');
    let content = std::fs::read_to_string(config_file)
        .with_context(|| format!("无法读取 {}", config_file))?;
    let re = Regex::new(pattern)?;
    let new_content = re.replace(&content, &replacement_fn(version));
    std::fs::write(config_file, new_content.as_ref())
        .with_context(|| format!("无法写入 {}", config_file))?;
    Ok(())
}

fn write_lines(config_file: &str, lines: &[String], trailing_newline: bool) -> Result<()> {
    let mut content = lines.join("\n");
    if trailing_newline {
        content.push('\n');
    }
    std::fs::write(config_file, content).with_context(|| format!("无法写入 {}", config_file))?;
    Ok(())
}
