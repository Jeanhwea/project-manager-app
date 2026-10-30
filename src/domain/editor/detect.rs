//! 配置文件的发现、读取与改写编排：决定 release 要改哪些文件、怎么改，
//! 以及在改完配置文件后再顺带更新哪些 lockfile 并把它们加入 git。

use super::{EditorRegistry, FileEditor};
use crate::domain::git::{GitOperation, ReleaseError};
use crate::error::Result;
use crate::model::operation::ShellOperation;
use crate::model::plan::{AddOperation, DisplayMessage};
use crate::utils;
use regex::Regex;
use std::path::{Path, PathBuf};

/// 明确指定文件时只保留能被某个编辑器识别的；未指定则改为自动探测。
pub fn resolve_config_files(registry: &EditorRegistry, files: &[String]) -> Result<Vec<String>> {
    if files.is_empty() {
        return detect_config_files(registry);
    }

    files
        .iter()
        .filter(|f| registry.detect_editor(Path::new(f)).is_some())
        .cloned()
        .map(Ok)
        .collect()
}

/// 按注册表给出的候选路径探测仓库中真实存在的配置文件，一个都找不到时报错。
pub fn detect_config_files(registry: &EditorRegistry) -> Result<Vec<String>> {
    let mut result = Vec::new();

    for candidate in registry.candidate_files() {
        if candidate.contains("{}") {
            for path in expand_glob_pattern(candidate) {
                if Path::new(&path).exists() && registry.detect_editor(Path::new(&path)).is_some()
                {
                    result.push(path);
                }
            }
        } else if Path::new(candidate).exists()
            && registry.detect_editor(Path::new(candidate)).is_some()
        {
            result.push(candidate.to_string());
        }
    }

    if result.is_empty() {
        return Err(ReleaseError::NoConfigFiles.into());
    }

    Ok(result)
}

/// 把形如 `{}/package.json` 的候选模板展开成目录中各子目录下的实际路径。
pub fn expand_glob_pattern(pattern: &str) -> Vec<String> {
    let mut results = Vec::new();
    let (prefix, suffix) = match pattern.split_once("{}") {
        Some(pair) => pair,
        None => return results,
    };

    // 模板以 {} 开头时扫当前目录，否则扫前缀指定的目录
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
            // 隐藏目录与 node_modules 里的包不是本项目成员，跳过以免误改依赖的版本号
            if dir_name.starts_with('.') || dir_name == "node_modules" {
                continue;
            }
            results.push(format!("{}{}{}", prefix, dir_name, suffix));
        }
    }

    results
}

/// 改完配置文件后，把对应生态的 lockfile 更新操作追加到执行计划中。
pub fn add_lockfile_operations(plan: &mut impl AddOperation, config_file: &str) {
    if config_file.ends_with("Cargo.toml") {
        add_cargo_lock_operations(plan, config_file);
    } else if config_file.ends_with("package.json") {
        add_js_lockfile_operations(plan, config_file);
    } else if config_file.ends_with("pyproject.toml") {
        add_uv_lock_operations(plan, config_file);
    }
}

fn add_uv_lock_operations(plan: &mut impl AddOperation, pyproject_path: &str) {
    use crate::domain::git::release::is_gitignored;

    let dir = parent_dir(Path::new(pyproject_path));
    let lock_path = dir.join("uv.lock");

    // lockfile 不存在或被 gitignore 时不处理：无需更新，也不该被 git add
    if !lock_path.exists() || is_gitignored(&lock_path) {
        return;
    }

    if !utils::is_command_available("uv") {
        plan.add_msg(DisplayMessage::Warning {
            msg: "未检测到 uv 命令，跳过 uv.lock 更新".to_string(),
        });
        return;
    }

    plan.add_op(ShellOperation::Run {
        program: "uv".to_string(),
        args: vec!["lock".to_string()],
        dir: Some(dir.to_path_buf()),
        description: "uv lock".to_string(),
        optional: true,
    });

    let path_str = lock_path.to_string_lossy().replace('\\', "/");
    plan.add_op(GitOperation::Add {
        path: path_str,
        working_dir: PathBuf::from("."),
    });
}

fn add_cargo_lock_operations(plan: &mut impl AddOperation, cargo_toml_path: &str) {
    use crate::domain::git::release::is_gitignored;

    let dir = parent_dir(Path::new(cargo_toml_path));
    let lock_path = dir.join("Cargo.lock");

    if !lock_path.exists() || is_gitignored(&lock_path) {
        return;
    }

    // 能拿到包名就精确更新该包；workspace 根文件没有包名，退化为整体更新
    if let Ok(package_name) = read_cargo_package_name(cargo_toml_path) {
        plan.add_op(ShellOperation::Run {
            program: "cargo".to_string(),
            args: vec![
                "update".to_string(),
                "--package".to_string(),
                package_name.clone(),
            ],
            dir: Some(dir.to_path_buf()),
            description: format!("cargo update --package {}", package_name),
            optional: true,
        });
    } else if is_cargo_workspace_root(cargo_toml_path) {
        plan.add_op(ShellOperation::Run {
            program: "cargo".to_string(),
            args: vec!["update".to_string(), "--workspace".to_string()],
            dir: Some(dir.to_path_buf()),
            description: "cargo update --workspace".to_string(),
            optional: true,
        });
    } else {
        return;
    }

    let path_str = lock_path.to_string_lossy().replace('\\', "/");
    plan.add_op(GitOperation::Add {
        path: path_str,
        working_dir: PathBuf::from("."),
    });
}

fn is_cargo_workspace_root(cargo_toml_path: &str) -> bool {
    let Ok(content) = std::fs::read_to_string(cargo_toml_path) else {
        return false;
    };
    content
        .parse::<toml_edit::DocumentMut>()
        .map(|doc| doc.contains_key("workspace"))
        .unwrap_or(false)
}

fn add_js_lockfile_operations(plan: &mut impl AddOperation, package_json_path: &str) {
    use crate::domain::git::release::is_gitignored;

    let pkg_dir = parent_dir(Path::new(package_json_path));

    if try_existing_js_lockfile(plan, pkg_dir, &is_gitignored) {
        return;
    }

    add_pnpm_fallback(plan, pkg_dir, &is_gitignored);
}

fn try_existing_js_lockfile(
    plan: &mut impl AddOperation,
    pkg_dir: &Path,
    is_gitignored: &dyn Fn(&Path) -> bool,
) -> bool {
    let lockfiles: &[(&str, &str, &[&str])] = &[
        ("pnpm-lock.yaml", "pnpm", &["install", "--lockfile-only"]),
        (
            "yarn.lock",
            "yarn",
            &["install", "--mode", "update-lockfile"],
        ),
        (
            "package-lock.json",
            "npm",
            &["install", "--package-lock-only"],
        ),
        ("bun.lock", "bun", &["install"]),
    ];

    // 按 lockfile 类型挑工具；命中第一个真实存在的 lockfile 后就不再考虑其它
    for (lock_name, cmd, args) in lockfiles {
        let lock_path = pkg_dir.join(lock_name);
        if !lock_path.exists() || is_gitignored(&lock_path) {
            continue;
        }
        if !utils::is_command_available(cmd) {
            plan.add_msg(DisplayMessage::Warning {
                msg: format!("未检测到 {} 命令，跳过 {} 更新", cmd, lock_name),
            });
            return true;
        }
        let args_vec: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        plan.add_op(ShellOperation::Run {
            program: cmd.to_string(),
            args: args_vec,
            dir: Some(pkg_dir.to_path_buf()),
            description: format!("{} {}", cmd, args.join(" ")),
            optional: true,
        });

        let path_str = lock_path.to_string_lossy().replace('\\', "/");
        plan.add_op(GitOperation::Add {
            path: path_str,
            working_dir: PathBuf::from("."),
        });
        return true;
    }
    false
}

fn add_pnpm_fallback(
    plan: &mut impl AddOperation,
    pkg_dir: &Path,
    is_gitignored: &dyn Fn(&Path) -> bool,
) {
    if utils::is_command_available("pnpm") {
        let lock_path = pkg_dir.join("pnpm-lock.yaml");
        if !is_gitignored(&lock_path) {
            plan.add_op(ShellOperation::Run {
                program: "pnpm".to_string(),
                args: vec!["install".to_string(), "--lockfile-only".to_string()],
                dir: Some(pkg_dir.to_path_buf()),
                description: "pnpm install --lockfile-only".to_string(),
                optional: true,
            });
        }
    }
    #[cfg(target_os = "windows")]
    let warning_msg = "未检测到 pnpm 命令，跳过 pnpm lockfile 更新。在 Windows 环境中，建议安装 pnpm 或使用 npm";
    #[cfg(not(target_os = "windows"))]
    let warning_msg = "未检测到 pnpm 命令，跳过 pnpm lockfile 更新";

    plan.add_msg(DisplayMessage::Warning {
        msg: warning_msg.to_string(),
    });
}

/// 解析 tag、读取文件、改版本号并校验结果，返回改写前后的完整内容。
pub fn compute_edited_content(
    editor: &dyn FileEditor,
    tag: &str,
    config_file: &str,
) -> Result<(String, String)> {
    // tag 可能带 v 前缀，配置文件里的版本号不带
    let version = tag.trim_start_matches('v');
    let content = std::fs::read_to_string(config_file).map_err(|e| ReleaseError::ReadFile {
        path: config_file.to_string(),
        source: e,
    })?;

    let location = editor.parse(&content)?;
    let edited = editor.edit(&content, &location, version)?;
    editor.validate(&content, &edited)?;

    Ok((content, edited))
}

/// 读取某个配置文件里当前的版本号字符串（原文切片，未做规范化）。
pub fn read_file_version(editor: &dyn FileEditor, config_file: &str) -> Result<String> {
    let content = std::fs::read_to_string(config_file).map_err(|e| ReleaseError::ReadFile {
        path: config_file.to_string(),
        source: e,
    })?;
    let location = editor.parse(&content)?;
    let pos =
        location
            .project_version
            .as_ref()
            .ok_or_else(|| ReleaseError::VersionFieldNotFound {
                path: config_file.to_string(),
            })?;
    let version_str = &content[pos.start..pos.end];
    Ok(version_str.to_string())
}

/// 在多个配置文件中取最大的版本号，作为没有 tag 可参照时的兜底基准。
pub fn extract_fallback_version(
    registry: &EditorRegistry,
    config_files: &[String],
) -> Option<String> {
    use super::Version;

    let mut best: Option<Version> = None;
    for file_path in config_files {
        // 注意：任何一个文件匹配不到编辑器都会让整个函数返回 None，而不是跳过该文件
        let editor = registry.detect_editor(Path::new(file_path))?;
        if let Ok(ver_str) = read_file_version(editor, file_path)
            && let Ok(ver) = Version::parse(&ver_str)
            && best.as_ref().is_none_or(|b| ver > *b)
        {
            best = Some(ver);
        }
    }
    best.map(|v| v.to_string())
}

/// 从 Cargo.toml 的 [package] 段读取包名，供 cargo update --package 使用。
pub fn read_cargo_package_name(cargo_toml_path: &str) -> Result<String> {
    let content =
        std::fs::read_to_string(cargo_toml_path).map_err(|e| ReleaseError::ReadFile {
            path: cargo_toml_path.to_string(),
            source: e,
        })?;
    let re = Regex::new(r#"name\s*=\s*"([^"]*)""#)?;
    // 逐行跟踪当前是否仍在 [package] 段内：遇到下一个 [section] 就退出，
    // 否则会取到 [dependencies] 等段里的 name
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
    Err(ReleaseError::PackageNameNotFound {
        path: cargo_toml_path.to_string(),
    }
    .into())
}

fn parent_dir(path: &Path) -> &Path {
    // 形如 "Cargo.toml" 的相对路径没有 parent，退回到当前目录
    let parent = path.parent().unwrap_or(Path::new("."));
    if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_expand_glob_pattern() {
        let temp_dir = tempdir().unwrap();
        let dir1 = temp_dir.path().join("dir1");
        let dir2 = temp_dir.path().join("dir2");
        std::fs::create_dir(&dir1).unwrap();
        std::fs::create_dir(&dir2).unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let expanded = expand_glob_pattern("{}/package.json");

        assert!(expanded.contains(&"dir1/package.json".to_string()));
        assert!(expanded.contains(&"dir2/package.json".to_string()));

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_read_cargo_package_name() {
        let temp_dir = tempdir().unwrap();
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");

        std::fs::write(
            &cargo_toml_path,
            r#"[package]
name = "test-package"
version = "0.1.0"

[dependencies]
serde = "1.0""#,
        )
        .unwrap();

        let package_name = read_cargo_package_name(&cargo_toml_path.to_string_lossy()).unwrap();
        assert_eq!(package_name, "test-package");
    }

    #[test]
    fn test_read_cargo_package_name_not_found() {
        let temp_dir = tempdir().unwrap();
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");

        std::fs::write(
            &cargo_toml_path,
            r#"[dependencies]
serde = "1.0""#,
        )
        .unwrap();

        assert!(read_cargo_package_name(&cargo_toml_path.to_string_lossy()).is_err());
    }
}
