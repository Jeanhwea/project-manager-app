use crate::domain::git::{GitOperation, is_gitignored};
use crate::domain::git::ReleaseError;
use crate::error::Result;
use crate::model::operation::ShellOperation;
use crate::model::plan::{AddOperation, DisplayMessage};
use crate::utils;
use regex::Regex;
use std::path::{Path, PathBuf};

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
    let dir = parent_dir(Path::new(pyproject_path));
    let lock_path = dir.join("uv.lock");

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
    let dir = parent_dir(Path::new(cargo_toml_path));
    let lock_path = dir.join("Cargo.lock");

    if !lock_path.exists() || is_gitignored(&lock_path) {
        return;
    }

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

pub fn read_cargo_package_name(cargo_toml_path: &str) -> Result<String> {
    let content =
        std::fs::read_to_string(cargo_toml_path).map_err(|e| ReleaseError::ReadFile {
            path: cargo_toml_path.to_string(),
            source: e,
        })?;
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
    Err(ReleaseError::PackageNameNotFound {
        path: cargo_toml_path.to_string(),
    }
    .into())
}

fn parent_dir(path: &Path) -> &Path {
    let parent = path.parent().unwrap_or(Path::new("."));
    if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    }
}
