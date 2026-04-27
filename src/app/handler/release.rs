use crate::app::common::editor::{
    CMakeListsEditor, CargoTomlEditor, EditorRegistry, HomebrewFormulaEditor, PackageJsonEditor,
    PomXmlEditor, PyprojectEditor, PythonVersionEditor, VersionTextEditor, write_with_backup,
};
use crate::app::common::git;
use crate::app::common::runner::DryRunContext;
use crate::app::common::version::Version;
use crate::utils;
use anyhow::{Context, Result};
use colored::Colorize;
use regex::Regex;
use std::path::Path;

const CONFIG_FILE_CANDIDATES: &[(&str, bool)] = &[
    ("Cargo.toml", false),
    ("src-tauri/Cargo.toml", false),
    ("pom.xml", false),
    ("pyproject.toml", false),
    ("{}/__version__.py", true),
    ("version", false),
    ("version.txt", false),
    ("package.json", false),
    ("apps/{}/package.json", true),
    ("ui/package.json", false),
    ("src-tauri/tauri.conf.json", false),
    ("npm/{}/package.json", true),
    ("CMakeLists.txt", false),
    ("Formula/pma.rb", false),
];

type ConfigFileEntry = (
    String,
    std::sync::Arc<dyn crate::app::common::editor::ConfigEditor>,
);

struct GitState {
    current_branch: String,
    new_tag: String,
}

fn create_editor_registry() -> EditorRegistry {
    EditorRegistry::new()
        .register(CargoTomlEditor)
        .register(PomXmlEditor)
        .register(PyprojectEditor)
        .register(VersionTextEditor)
        .register(CMakeListsEditor)
        .register(HomebrewFormulaEditor)
        .register(PythonVersionEditor)
        .register(PackageJsonEditor { in_npm_dir: false })
}

pub fn execute(
    bump_type: &str,
    files: &[String],
    no_root: bool,
    force: bool,
    skip_push: bool,
    dry_run: bool,
) -> Result<()> {
    let ctx = DryRunContext::new(dry_run);
    let resolved_files = resolve_file_paths(files);
    switch_to_git_root(no_root);
    let state = validate_git_state(force, bump_type)?;
    let registry = create_editor_registry();
    let config_files = resolve_config_files(&registry, &resolved_files)?;

    if ctx.is_dry_run() {
        execute_dry_run(&ctx, &registry, &config_files, &state, skip_push)
    } else {
        execute_release(&registry, &config_files, &state, skip_push)
    }
}

fn resolve_file_paths(files: &[String]) -> Vec<String> {
    files
        .iter()
        .map(|f| {
            if Path::new(f).is_absolute() {
                f.clone()
            } else {
                utils::canonicalize_path(f)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| f.clone())
            }
        })
        .collect()
}

fn switch_to_git_root(no_root: bool) {
    if !no_root
        && let Some(root) = git::get_top_level_dir()
        && let Err(e) = std::env::set_current_dir(&root)
    {
        eprintln!("警告: 无法切换到 git 根目录: {}, {}", root.display(), e);
    }
}

fn validate_git_state(force: bool, bump_type: &str) -> Result<GitState> {
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

    println!(
        "{} {} -> {}",
        "版本变更:".green().bold(),
        current_tag.cyan(),
        new_tag.yellow().bold()
    );

    Ok(GitState {
        current_branch,
        new_tag,
    })
}

fn resolve_config_files(
    registry: &EditorRegistry,
    files: &[String],
) -> Result<Vec<ConfigFileEntry>> {
    if files.is_empty() {
        return detect_config_files(registry);
    }

    files
        .iter()
        .map(|f| {
            let path = Path::new(f);
            let editor = registry
                .detect_editor(path)
                .with_context(|| format!("无法识别文件类型: {}", f))?;
            Ok((f.clone(), editor))
        })
        .collect()
}

fn execute_dry_run(
    ctx: &DryRunContext,
    registry: &EditorRegistry,
    config_files: &[ConfigFileEntry],
    state: &GitState,
    skip_push: bool,
) -> Result<()> {
    ctx.print_header("\n[DRY-RUN] 将要修改的文件:");
    for (file_path, editor) in config_files {
        print_file_diff(ctx, registry, editor.as_ref(), &state.new_tag, file_path)?;
    }

    ctx.print_header("\n[DRY-RUN] 将要执行的操作:");
    for (file_path, _editor) in config_files {
        print_lock_update_plan(ctx, file_path);
    }
    ctx.print_message("git add <files>");
    ctx.print_message(&format!("git commit -m \"{}\"", state.new_tag));
    ctx.print_message(&format!("git tag {}", state.new_tag));
    print_push_plan(ctx, &state.current_branch, &state.new_tag, skip_push);
    Ok(())
}

fn execute_release(
    registry: &EditorRegistry,
    config_files: &[ConfigFileEntry],
    state: &GitState,
    skip_push: bool,
) -> Result<()> {
    for (file_path, editor) in config_files {
        edit_version_in_file(registry, editor.as_ref(), &state.new_tag, file_path)?;
        post_edit_version_file(file_path)?;
        git::add_file(file_path)?;
    }

    git::list_cached_changes()?;
    git::commit(&state.new_tag)?;
    git::create_tag(&state.new_tag)?;
    push_to_remotes(skip_push, &state.current_branch, &state.new_tag);
    Ok(())
}

fn print_push_plan(ctx: &DryRunContext, current_branch: &str, new_tag: &str, skip_push: bool) {
    if !skip_push && let Some(remotes) = git::get_remote_list() {
        for remote in remotes {
            ctx.print_message(&format!("git push {} {}", remote, new_tag));
            ctx.print_message(&format!("git push {} {}", remote, current_branch));
        }
    }
}

fn print_lock_update_plan(ctx: &DryRunContext, config_file: &str) {
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

fn push_to_remotes(skip_push: bool, current_branch: &str, new_tag: &str) {
    if skip_push {
        return;
    }
    if let Some(remotes) = git::get_remote_list() {
        for remote in remotes {
            if let Err(e) = git::push_tag(&remote, new_tag) {
                eprintln!("警告: 推送标签失败: {}", e);
            }
            if let Err(e) = git::push_branch(&remote, current_branch) {
                eprintln!("警告: 推送分支失败: {}", e);
            }
        }
    }
}

fn compute_edited_content(
    registry: &EditorRegistry,
    editor: &dyn crate::app::common::editor::ConfigEditor,
    tag: &str,
    config_file: &str,
) -> Result<(String, String)> {
    let version = tag.trim_start_matches('v');
    let content = std::fs::read_to_string(config_file)
        .with_context(|| format!("无法读取 {}", config_file))?;

    let in_npm_dir = config_file.starts_with("npm/");
    let edited = if editor.name() == "package_json" {
        let pkg_editor = PackageJsonEditor { in_npm_dir };
        registry.edit_version(&pkg_editor, &content, version)?
    } else {
        registry.edit_version(editor, &content, version)?
    };

    Ok((content, edited))
}

fn detect_config_files(registry: &EditorRegistry) -> Result<Vec<ConfigFileEntry>> {
    let mut result = Vec::new();

    for (pattern, _is_dynamic) in CONFIG_FILE_CANDIDATES {
        if pattern.contains("{}") {
            for path in expand_glob_pattern(pattern) {
                if Path::new(&path).exists()
                    && let Some(editor) = registry.detect_editor(Path::new(&path))
                {
                    result.push((path, editor));
                }
            }
        } else if Path::new(pattern).exists()
            && let Some(editor) = registry.detect_editor(Path::new(pattern))
        {
            result.push((pattern.to_string(), editor));
        }
    }

    if result.is_empty() {
        anyhow::bail!("未检测到可编辑的配置文件");
    }

    Ok(result)
}

fn expand_glob_pattern(pattern: &str) -> Vec<String> {
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
            let candidate = format!("{}{}{}", prefix, dir_name, suffix);
            results.push(candidate);
        }
    }

    results
}

fn edit_version_in_file(
    registry: &EditorRegistry,
    editor: &dyn crate::app::common::editor::ConfigEditor,
    tag: &str,
    config_file: &str,
) -> Result<()> {
    let (_original, edited) = compute_edited_content(registry, editor, tag, config_file)?;
    write_with_backup(config_file, &edited).map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

fn post_edit_version_file(config_file: &str) -> Result<()> {
    if config_file.ends_with("Cargo.toml") {
        update_cargo_lock(config_file)?;
    } else if config_file.ends_with("package.json") {
        update_npm_lock(config_file)?;
        update_pnpm_lock(config_file)?;
    }
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

fn update_npm_lock(package_json_path: &str) -> Result<()> {
    let parent = Path::new(package_json_path)
        .parent()
        .unwrap_or(Path::new("."));
    let dir = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };

    let lock_path = dir.join("package-lock.json");
    if !lock_path.exists() {
        return Ok(());
    }

    if !check_command_exists("npm") {
        eprintln!("警告: 未找到 npm 命令，跳过 package-lock.json 更新");
        return Ok(());
    }

    let status = std::process::Command::new("npm")
        .args(["install", "--package-lock-only"])
        .current_dir(dir)
        .status()
        .context("无法执行 npm install --package-lock-only")?;

    if !status.success() {
        anyhow::bail!("npm install --package-lock-only 执行失败");
    }

    let lock_str = lock_path.to_string_lossy().to_string();
    git::add_file(&lock_str)?;
    Ok(())
}

fn update_pnpm_lock(package_json_path: &str) -> Result<()> {
    let parent = Path::new(package_json_path)
        .parent()
        .unwrap_or(Path::new("."));
    let dir = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };

    let lock_path = dir.join("pnpm-lock.yaml");
    if !lock_path.exists() {
        return Ok(());
    }

    if !check_command_exists("pnpm") {
        eprintln!("警告: 未找到 pnpm 命令，跳过 pnpm-lock.yaml 更新");
        return Ok(());
    }

    let status = std::process::Command::new("pnpm")
        .args(["install", "--lockfile-only"])
        .current_dir(dir)
        .status()
        .context("无法执行 pnpm install --lockfile-only")?;

    if !status.success() {
        anyhow::bail!("pnpm install --lockfile-only 执行失败");
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

fn print_file_diff(
    ctx: &DryRunContext,
    registry: &EditorRegistry,
    editor: &dyn crate::app::common::editor::ConfigEditor,
    tag: &str,
    config_file: &str,
) -> Result<()> {
    let (original, edited) = compute_edited_content(registry, editor, tag, config_file)?;
    ctx.print_file_diff(config_file, &original, &edited);
    Ok(())
}
