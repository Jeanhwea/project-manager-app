use crate::control::context::collect_context;
use crate::control::plan::run_plan;
use crate::domain::AppError;
use crate::domain::editor::{BumpType, EditorRegistry, FileEditor, Version};
use crate::domain::git::GitCommandRunner;
use crate::model::git::GitContext;
use crate::model::plan::{
    EditOperation, ExecutionPlan, GitOperation, MessageOperation, ShellOperation,
};
use crate::utils::output::{ItemColor, Output};
use crate::utils::path::canonicalize_path;
use anyhow::Result;
use regex::Regex;
use std::path::Path;

#[derive(Debug, clap::Args)]
pub struct ReleaseArgs {
    #[arg(
        value_enum,
        default_value = "patch",
        help = "Bump type: major, minor, patch"
    )]
    pub bump_type: BumpType,
    #[arg(help = "Files to update version (auto-detect if not specified)")]
    pub files: Vec<String>,
    #[arg(
        long,
        short = 'n',
        default_value = "false",
        help = "Stay in current directory"
    )]
    pub no_root: bool,
    #[arg(
        long,
        short,
        default_value = "false",
        help = "Force release even if not on master"
    )]
    pub force: bool,
    #[arg(long, default_value = "false", help = "Skip pushing tags and branches")]
    pub skip_push: bool,
    #[arg(long, default_value = "false", help = "Dry run")]
    pub dry_run: bool,
    #[arg(long, short = 'm', help = "Custom commit message")]
    pub message: Option<String>,
    #[arg(long, help = "Pre-release suffix (e.g. \"alpha\" -> v1.0.0-alpha)")]
    pub pre_release: Option<String>,
}

struct GitState {
    current_branch: String,
    new_tag: String,
    commit_message: String,
}

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

pub fn run(args: ReleaseArgs) -> anyhow::Result<()> {
    let resolved_files = resolve_file_paths(&args.files);

    if !args.no_root {
        switch_to_git_root()?;
    }

    let ctx = collect_context(Path::new("."))?;
    let state = validate_git_state(&args, &ctx)?;
    let registry = EditorRegistry::default_with_editors();
    let config_files = resolve_config_files(&registry, &resolved_files)?;

    let mut plan = build_execution_plan(&args, &config_files, &state, &ctx, &registry);
    plan.dry_run = args.dry_run;
    run_plan(&plan)
}

fn resolve_file_paths(files: &[String]) -> Vec<String> {
    files
        .iter()
        .map(|f| {
            if Path::new(f).is_absolute() {
                f.clone().replace('\\', "/")
            } else {
                canonicalize_path(f)
                    .map(|p| p.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_else(|_| f.clone().replace('\\', "/"))
            }
        })
        .collect()
}

fn switch_to_git_root() -> Result<()> {
    let runner = GitCommandRunner::new();
    let root = runner.execute(&["rev-parse", "--show-toplevel"], None)?;
    if !root.is_empty() {
        std::env::set_current_dir(&root)
            .map_err(|e| anyhow::anyhow!("无法切换到 git 根目录: {} - {}", root, e))?;
    }
    Ok(())
}

fn validate_git_state(args: &ReleaseArgs, ctx: &GitContext) -> Result<GitState> {
    if !args.force && ctx.current_branch != "master" {
        return Err(AppError::release("只能在 master 分支上执行 release").into());
    }

    let runner = GitCommandRunner::new();
    let previous_tag = runner
        .execute(&["describe", "--tags", "--match", "v*"], None)
        .ok()
        .and_then(|o| o.split('-').next().map(|s| s.to_string()));
    let current_tag = previous_tag.clone().unwrap_or_else(|| "v0.0.0".to_string());

    if let Some(ref tag) = previous_tag {
        let rev_current_tag = runner.execute(&["rev-parse", tag], None)?;
        let rev_head = runner.execute(&["rev-parse", "HEAD"], None)?;
        if rev_current_tag.trim() == rev_head.trim() {
            return Err(AppError::release(format!("当前 HEAD 已被标记为 {}", tag)).into());
        }
    }

    let version = Version::from_tag(&current_tag).unwrap_or_default();
    let new_version = version.bump(&args.bump_type);
    let mut new_tag = new_version.to_tag();

    if let Some(pre) = &args.pre_release {
        new_tag = format!("{}-{}", new_tag, pre);
    }

    let commit_message = match &args.message {
        Some(msg) => format!("{} {}", new_tag, msg),
        None => new_tag.clone(),
    };

    Output::item_colored(
        &format!("版本变更: {} ->", current_tag),
        &new_tag,
        ItemColor::Yellow,
    );

    if args.message.is_some() {
        Output::item("提交消息", &commit_message);
    }

    Ok(GitState {
        current_branch: ctx.current_branch.clone(),
        new_tag,
        commit_message,
    })
}

fn resolve_config_files(registry: &EditorRegistry, files: &[String]) -> Result<Vec<String>> {
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

fn detect_config_files(registry: &EditorRegistry) -> Result<Vec<String>> {
    let mut result = Vec::new();

    for (pattern, is_dynamic) in CONFIG_FILE_CANDIDATES {
        if *is_dynamic && pattern.contains("{}") {
            for path in expand_glob_pattern(pattern) {
                if Path::new(&path).exists() && registry.detect_editor(Path::new(&path)).is_some()
                {
                    result.push(path);
                }
            }
        } else if Path::new(pattern).exists()
            && registry.detect_editor(Path::new(pattern)).is_some()
        {
            result.push(pattern.to_string());
        }
    }

    if result.is_empty() {
        return Err(AppError::release("未检测到可编辑的配置文件").into());
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
            results.push(format!("{}{}{}", prefix, dir_name, suffix));
        }
    }

    results
}

fn build_execution_plan(
    args: &ReleaseArgs,
    config_files: &[String],
    state: &GitState,
    ctx: &GitContext,
    registry: &EditorRegistry,
) -> ExecutionPlan {
    let mut plan = ExecutionPlan::new();

    plan.add(MessageOperation::Section {
        title: "修改计划".to_string(),
    });

    for file_path in config_files {
        let editor = registry.detect_editor(Path::new(file_path)).unwrap();
        if let Ok((original, edited)) = compute_edited_content(editor, &state.new_tag, file_path)
        {
            let old_lines: Vec<&str> = original.lines().collect();
            let new_lines: Vec<&str> = edited.lines().collect();
            for (line_num, (old_line, new_line)) in
                (1..).zip(old_lines.iter().zip(new_lines.iter()))
            {
                if old_line != new_line {
                    plan.add(MessageOperation::Diff {
                        file: file_path.clone(),
                        line_num,
                        old: old_line.to_string(),
                        new: new_line.to_string(),
                    });
                }
            }

            plan.add(EditOperation::WriteFile {
                path: file_path.clone(),
                content: edited,
                description: format!("edit {}", file_path),
            });

            add_lockfile_operations(&mut plan, file_path);

            plan.add(GitOperation::Add {
                path: file_path.clone(),
            });
        }
    }

    plan.add(GitOperation::Commit {
        message: state.commit_message.clone(),
    });
    plan.add(GitOperation::CreateTag {
        tag: state.new_tag.clone(),
    });

    if !args.skip_push {
        for remote in ctx.remote_names() {
            plan.add(GitOperation::PushTag {
                remote: remote.to_string(),
                tag: state.new_tag.clone(),
            });
            plan.add(GitOperation::PushBranch {
                remote: remote.to_string(),
                branch: state.current_branch.clone(),
            });
        }
    }

    plan
}

fn add_lockfile_operations(plan: &mut ExecutionPlan, config_file: &str) {
    if config_file.ends_with("Cargo.toml") {
        add_cargo_lock_operations(plan, config_file);
    } else if config_file.ends_with("package.json") {
        add_js_lockfile_operations(plan, config_file);
    }
}

fn add_cargo_lock_operations(plan: &mut ExecutionPlan, cargo_toml_path: &str) {
    let dir = parent_dir(Path::new(cargo_toml_path));
    let lock_path = dir.join("Cargo.lock");

    if !lock_path.exists() || is_gitignored(&lock_path) {
        return;
    }

    let Ok(pkg_name) = read_cargo_package_name(cargo_toml_path) else {
        return;
    };

    plan.add(ShellOperation::Run {
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
            plan.add(ShellOperation::Run {
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
            plan.add(ShellOperation::Run {
                program: "pnpm".to_string(),
                args: vec!["install".to_string(), "--lockfile-only".to_string()],
                dir: Some(pkg_dir.to_path_buf()),
                description: "pnpm install --lockfile-only".to_string(),
            });
        }
    }
}

fn compute_edited_content(
    editor: &dyn FileEditor,
    tag: &str,
    config_file: &str,
) -> Result<(String, String)> {
    let version = tag.trim_start_matches('v');
    let content = std::fs::read_to_string(config_file)
        .map_err(|e| anyhow::anyhow!("无法读取 {}: {}", config_file, e))?;

    let location = editor.parse(&content)?;
    let edited = editor.edit(&content, &location, version)?;
    editor.validate(&content, &edited)?;

    Ok((content, edited))
}

fn parent_dir(path: &Path) -> &Path {
    let parent = path.parent().unwrap_or(Path::new("."));
    if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    }
}

fn is_gitignored(file_path: &Path) -> bool {
    let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let Some(parent) = file_path.parent() else {
        return false;
    };

    let runner = GitCommandRunner::new();
    let output = runner.execute_raw(&["check-ignore", file_name], parent);

    match output {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

fn read_cargo_package_name(cargo_toml_path: &str) -> Result<String> {
    let content = std::fs::read_to_string(cargo_toml_path)
        .map_err(|e| anyhow::anyhow!("无法读取 {}: {}", cargo_toml_path, e))?;
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
    Err(AppError::release(format!("未在 {} 中找到 [package] name", cargo_toml_path)).into())
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
