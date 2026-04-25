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
        &[
            "package.json",
            "apps/{}/package.json",
            "ui/package.json",
            "src-tauri/tauri.conf.json",
            "npm/{}/package.json",
        ],
        ConfigFileType::PackageJson,
    ),
    (&["CMakeLists.txt"], ConfigFileType::CMakeLists),
    (&["Formula/pma.rb"], ConfigFileType::HomebrewFormula),
];

#[derive(Clone, Copy)]
enum ConfigFileType {
    CargoToml,
    PomXml,
    PyprojectToml,
    PythonVersion,
    VersionText,
    PackageJson,
    CMakeLists,
    HomebrewFormula,
}

pub fn execute(
    bump_type: &str,
    files: &[String],
    no_root: bool,
    force: bool,
    skip_push: bool,
) -> Result<()> {
    // 除非指定 --no-root，否则先切换到 git 仓库根目录
    if !no_root
        && let Some(root) = git::get_top_level_dir()
        && let Err(e) = std::env::set_current_dir(&root)
    {
        eprintln!("警告: 无法切换到 git 根目录: {}, {}", root.display(), e);
    }

    let current_branch = git::get_current_branch().unwrap_or_else(|| "master".to_string());
    if !force && current_branch != "master" {
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

    let config_files = if files.is_empty() {
        detect_config_files()?
    } else {
        files
            .iter()
            .map(|f| {
                let file_type = detect_file_type(f)?;
                Ok((f.clone(), file_type))
            })
            .collect::<Result<Vec<_>>>()?
    };

    for (file_path, file_type) in &config_files {
        edit_version_in_file(&new_tag, file_path, *file_type)?;
        post_edit_version_file(file_path, *file_type)?;
        git::add_file(file_path)?;
    }

    git::list_cached_changes()?;
    git::commit(&new_tag)?;
    git::create_tag(&new_tag)?;

    if !skip_push && let Some(remotes) = git::get_remote_list() {
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

fn detect_file_type(file_path: &str) -> Result<ConfigFileType> {
    let path = Path::new(file_path);
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    let parent_dir = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("");

    let file_type = match file_name {
        "Cargo.toml" => ConfigFileType::CargoToml,
        "pom.xml" => ConfigFileType::PomXml,
        "pyproject.toml" => ConfigFileType::PyprojectToml,
        "__version__.py" => ConfigFileType::PythonVersion,
        "version" | "version.txt" => ConfigFileType::VersionText,
        "package.json" => ConfigFileType::PackageJson,
        "tauri.conf.json" => ConfigFileType::PackageJson,
        "CMakeLists.txt" => ConfigFileType::CMakeLists,
        "pma.rb" if parent_dir == "Formula" => ConfigFileType::HomebrewFormula,
        _ => {
            if file_name.ends_with(".py") {
                ConfigFileType::PythonVersion
            } else {
                anyhow::bail!("无法识别文件类型: {}", file_path);
            }
        }
    };

    Ok(file_type)
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
        ConfigFileType::CMakeLists => {
            return edit_with_regex(
                config_file,
                tag,
                r#"(project\s*\([^)]*?VERSION\s+)[0-9]+\.[0-9]+\.[0-9]+"#,
                |v| format!("${{1}}{}", v),
            );
        }
        ConfigFileType::HomebrewFormula => {
            return edit_with_regex(config_file, tag, r#"version "[^"]*""#, |v| {
                format!(r#"version "{}""#, v)
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
            if config_file.starts_with("npm/") {
                let re_dep = Regex::new(r#"("@jeansoft/pma[^"]*":\s*")[^"]*""#)?;
                let new_content =
                    re.replace_all(&content, &format!(r#""version": "{}""#, version));
                let new_content =
                    re_dep.replace_all(&new_content, &format!(r#"${{1}}{}""#, version));
                std::fs::write(config_file, new_content.as_ref())
                    .with_context(|| format!("无法写入 {}", config_file))?;
                return Ok(());
            }
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

fn post_edit_version_file(config_file: &str, file_type: ConfigFileType) -> Result<()> {
    match file_type {
        ConfigFileType::CargoToml => update_cargo_lock(config_file),
        _ => Ok(()),
    }
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

fn check_command_exists(cmd: &str) -> bool {
    std::process::Command::new(cmd)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

fn update_cargo_lock(cargo_toml_path: &str) -> Result<()> {
    let parent = Path::new(cargo_toml_path)
        .parent()
        .unwrap_or(Path::new("."));
    let dir = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };

    let lock_path = dir.join("Cargo.lock");
    if !lock_path.exists() {
        return Ok(());
    }

    if !check_command_exists("cargo") {
        anyhow::bail!("未找到 cargo 命令，请先安装 Rust 工具链: https://rustup.rs");
    }

    let pkg_name = read_cargo_package_name(cargo_toml_path)?;
    let status = std::process::Command::new("cargo")
        .args(["update", "--package", &pkg_name])
        .current_dir(dir)
        .status()
        .context("无法执行 cargo update")?;

    if !status.success() {
        anyhow::bail!("cargo update --package {} 执行失败", pkg_name);
    }

    let lock_str = lock_path.to_string_lossy().to_string();
    git::add_file(&lock_str)?;
    Ok(())
}

fn read_cargo_package_name(cargo_toml_path: &str) -> Result<String> {
    let content = std::fs::read_to_string(cargo_toml_path)
        .with_context(|| format!("无法读取 {}", cargo_toml_path))?;
    let re = Regex::new(r#"name\s*=\s*"([^"]*)""#)?;
    let mut in_package = false;
    for line in content.lines() {
        if line.trim() == "[package]" {
            in_package = true;
        } else if line.starts_with('[') {
            in_package = false;
        }
        if in_package && let Some(caps) = re.captures(line) {
            return Ok(caps[1].to_string());
        }
    }
    anyhow::bail!("未在 {} 中找到 [package] name", cargo_toml_path)
}
