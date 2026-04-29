use crate::app::common::git;
use crate::app::common::git::{check_command_exists, run_command};
use crate::app::common::runner::CommandRunner;
use anyhow::{Context, Result};
use colored::Colorize;
use regex::Regex;
use std::path::Path;

pub fn update_after_edit(config_file: &str) -> Result<()> {
    if config_file.ends_with("Cargo.toml") {
        update_cargo_lock(config_file)?;
    } else if config_file.ends_with("package.json") {
        update_npm_lock(config_file)?;
        update_pnpm_lock(config_file)?;
    }
    Ok(())
}

pub fn print_update_plan(ctx: &crate::app::common::runner::DryRunContext, config_file: &str) {
    let parent = Path::new(config_file).parent().unwrap_or(Path::new("."));
    let dir = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };

    if config_file.ends_with("Cargo.toml") && dir.join("Cargo.lock").exists() {
        ctx.print_message("cargo update --package <name>");
    } else if config_file.ends_with("package.json") {
        if dir.join("package-lock.json").exists() {
            ctx.print_message("npm install --package-lock-only");
        }
        if dir.join("pnpm-lock.yaml").exists() {
            ctx.print_message("pnpm install --lockfile-only");
        }
    }
}

fn is_gitignored(file_path: &Path) -> bool {
    let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let Some(parent) = file_path.parent() else {
        return false;
    };

    let Ok(output) = CommandRunner::run_quiet_in_dir("git", &["check-ignore", file_name], parent)
    else {
        return false;
    };

    output.status.success()
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

    if is_gitignored(&lock_path) {
        println!(
            "  {} Cargo.lock 在 .gitignore 中，跳过更新",
            "[SKIP]".dimmed()
        );
        return Ok(());
    }

    if !check_command_exists("cargo") {
        anyhow::bail!("未找到 cargo 命令，请先安装 Rust 工具链: https://rustup.rs");
    }

    let pkg_name = read_cargo_package_name(cargo_toml_path)?;
    let status = run_command("cargo", &["update", "--package", &pkg_name], dir)?;

    if !status.success() {
        anyhow::bail!("cargo update --package {} 执行失败", pkg_name);
    }

    let lock_str = lock_path.to_string_lossy().to_string();
    git::add_file(&lock_str)?;
    Ok(())
}

fn update_npm_lock(package_json_path: &str) -> Result<()> {
    let parent = Path::new(package_json_path)
        .parent()
        .unwrap_or(Path::new("."));
    let pkg_dir = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };

    let lock_dir = find_lock_dir(pkg_dir, "package-lock.json");
    let Some(lock_dir) = lock_dir else {
        return Ok(());
    };

    let lock_path = lock_dir.join("package-lock.json");

    if is_gitignored(&lock_path) {
        println!(
            "  {} package-lock.json 在 .gitignore 中，跳过更新",
            "[SKIP]".dimmed()
        );
        return Ok(());
    }

    if !check_command_exists("npm") {
        eprintln!("警告: 未找到 npm 命令，跳过 package-lock.json 更新");
        return Ok(());
    }

    let status = run_command("npm", &["install", "--package-lock-only"], &lock_dir)?;

    if !status.success() {
        eprintln!("npm install --package-lock-only 执行失败");
    }

    let lock_str = lock_path.to_string_lossy().to_string();
    git::add_file(&lock_str)?;
    Ok(())
}

fn update_pnpm_lock(package_json_path: &str) -> Result<()> {
    let parent = Path::new(package_json_path)
        .parent()
        .unwrap_or(Path::new("."));
    let pkg_dir = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };

    let lock_dir = find_lock_dir(pkg_dir, "pnpm-lock.yaml");
    let Some(lock_dir) = lock_dir else {
        return Ok(());
    };

    let lock_path = lock_dir.join("pnpm-lock.yaml");

    if is_gitignored(&lock_path) {
        println!(
            "  {} pnpm-lock.yaml 在 .gitignore 中，跳过更新",
            "[SKIP]".dimmed()
        );
        return Ok(());
    }

    if !check_command_exists("pnpm") {
        eprintln!("警告: 未找到 pnpm 命令，跳过 pnpm-lock.yaml 更新");
        return Ok(());
    }

    let status = run_command("pnpm", &["install", "--lockfile-only"], &lock_dir)?;

    if !status.success() {
        eprintln!("pnpm install --lockfile-only 执行失败");
    }

    let lock_str = lock_path.to_string_lossy().to_string();
    git::add_file(&lock_str)?;
    Ok(())
}

fn find_lock_dir(pkg_dir: &Path, lock_file: &str) -> Option<std::path::PathBuf> {
    if pkg_dir.join(lock_file).exists() {
        return Some(pkg_dir.to_path_buf());
    }
    let cwd = std::env::current_dir().ok()?;
    if cwd.join(lock_file).exists() {
        return Some(cwd);
    }
    None
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
