mod lockfile;

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
    commit_message: String,
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

#[allow(clippy::too_many_arguments)]
pub fn execute(
    bump_type: &str,
    files: &[String],
    no_root: bool,
    force: bool,
    skip_push: bool,
    dry_run: bool,
    message: Option<&str>,
    pre_release: Option<&str>,
) -> Result<()> {
    let ctx = DryRunContext::new(dry_run);
    let resolved_files = resolve_file_paths(files);
    switch_to_git_root(no_root);
    let state = validate_git_state(force, bump_type, message, pre_release)?;
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

fn validate_git_state(
    force: bool,
    bump_type: &str,
    message: Option<&str>,
    pre_release: Option<&str>,
) -> Result<GitState> {
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
    let mut new_tag = new_version.to_tag();

    if let Some(pre) = pre_release {
        new_tag = format!("{}-{}", new_tag, pre);
    }

    let commit_message = match message {
        Some(msg) => format!("{} {}", new_tag, msg),
        None => new_tag.clone(),
    };

    println!(
        "{} {} -> {}",
        "版本变更:".green().bold(),
        current_tag.cyan(),
        new_tag.yellow().bold()
    );

    if message.is_some() {
        println!("{} {}", "提交消息:".green().bold(), commit_message.yellow());
    }

    Ok(GitState {
        current_branch,
        new_tag,
        commit_message,
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
        lockfile::print_update_plan(ctx, file_path);
    }
    ctx.print_message("git add <files>");
    ctx.print_message(&format!("git commit -m \"{}\"", state.commit_message));
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
        lockfile::update_after_edit(file_path)?;
        git::add_file(file_path)?;
    }

    git::list_cached_changes()?;
    git::commit(&state.commit_message)?;
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
