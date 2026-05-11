use super::{EditorRegistry, FileEditor};
use crate::error::AppError;
use crate::model::plan::{ExecutionPlan, GitOperation};
use regex::Regex;
use std::path::Path;

pub fn resolve_config_files(
    registry: &EditorRegistry,
    files: &[String],
) -> crate::error::Result<Vec<String>> {
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

pub fn detect_config_files(registry: &EditorRegistry) -> crate::error::Result<Vec<String>> {
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
        return Err(AppError::release("未检测到可编辑的配置文件"));
    }

    Ok(result)
}

pub fn expand_glob_pattern(pattern: &str) -> Vec<String> {
    let mut results = Vec::new();
    let (prefix, suffix) = match pattern.split_once("{}") {
        Some(pair) => pair,
        None => return results,
    };

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
            if dir_name.starts_with('.') || dir_name == "node_modules" {
                continue;
            }
            results.push(format!("{}{}{}", prefix, dir_name, suffix));
        }
    }

    results
}

pub fn add_lockfile_operations(plan: &mut ExecutionPlan, config_file: &str) {
    if config_file.ends_with("Cargo.toml") {
        add_cargo_lock_operations(plan, config_file);
    } else if config_file.ends_with("package.json") {
        add_js_lockfile_operations(plan, config_file);
    }
}

fn add_cargo_lock_operations(plan: &mut ExecutionPlan, cargo_toml_path: &str) {
    use crate::domain::git::release::is_gitignored;

    let dir = parent_dir(Path::new(cargo_toml_path));
    let lock_path = dir.join("Cargo.lock");

    if !lock_path.exists() || is_gitignored(&lock_path) {
        return;
    }

    let Ok(pkg_name) = read_cargo_package_name(cargo_toml_path) else {
        return;
    };

    plan.add(crate::model::plan::ShellOperation::Run {
        program: "cargo".to_string(),
        args: vec![
            "update".to_string(),
            "--package".to_string(),
            pkg_name.clone(),
        ],
        dir: Some(dir.to_path_buf()),
        description: format!("cargo update --package {}", pkg_name),
    });

    let path_str = lock_path.to_string_lossy().replace('\\', "/");
    plan.add(GitOperation::Add { path: path_str });
}

fn add_js_lockfile_operations(plan: &mut ExecutionPlan, package_json_path: &str) {
    use crate::domain::git::release::is_gitignored;

    let pkg_dir = parent_dir(Path::new(package_json_path));

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
    ];

    for (lock_name, cmd, args) in lockfiles {
        let lock_path = pkg_dir.join(lock_name);
        if lock_path.exists() && !is_gitignored(&lock_path) {
            let args_vec: Vec<String> = args.iter().map(|s| s.to_string()).collect();
            plan.add(crate::model::plan::ShellOperation::Run {
                program: cmd.to_string(),
                args: args_vec,
                dir: Some(pkg_dir.to_path_buf()),
                description: format!("{} {}", cmd, args.join(" ")),
            });

            let path_str = lock_path.to_string_lossy().replace('\\', "/");
            plan.add(GitOperation::Add { path: path_str });
            return;
        }
    }

    if crate::utils::is_command_available("pnpm") {
        let lock_path = pkg_dir.join("pnpm-lock.yaml");
        if !is_gitignored(&lock_path) {
            plan.add(crate::model::plan::ShellOperation::Run {
                program: "pnpm".to_string(),
                args: vec!["install".to_string(), "--lockfile-only".to_string()],
                dir: Some(pkg_dir.to_path_buf()),
                description: "pnpm install --lockfile-only".to_string(),
            });
        }
    } else {
        #[cfg(target_os = "windows")]
        {
            plan.add(crate::model::plan::MessageOperation::Warning {
                msg: "未检测到 pnpm 命令，跳过 pnpm lockfile 更新。在 Windows 环境中，建议安装 pnpm 或使用 npm".to_string(),
            });
        }
        #[cfg(not(target_os = "windows"))]
        {
            plan.add(crate::model::plan::MessageOperation::Warning {
                msg: "未检测到 pnpm 命令，跳过 pnpm lockfile 更新".to_string(),
            });
        }
    }
}

pub fn compute_edited_content(
    editor: &dyn FileEditor,
    tag: &str,
    config_file: &str,
) -> crate::error::Result<(String, String)> {
    let version = tag.trim_start_matches('v');
    let content = std::fs::read_to_string(config_file)
        .map_err(|e| AppError::release(format!("无法读取 {}: {}", config_file, e)))?;

    let location = editor.parse(&content)?;
    let edited = editor.edit(&content, &location, version)?;
    editor.validate(&content, &edited)?;

    Ok((content, edited))
}

pub fn read_cargo_package_name(cargo_toml_path: &str) -> crate::error::Result<String> {
    let content = std::fs::read_to_string(cargo_toml_path)
        .map_err(|e| AppError::release(format!("无法读取 {}: {}", cargo_toml_path, e)))?;
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
    Err(AppError::release(format!(
        "未在 {} 中找到 [package] name",
        cargo_toml_path
    )))
}

fn parent_dir(path: &Path) -> &Path {
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
