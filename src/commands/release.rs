use crate::domain::AppError;
use crate::domain::editor::{BumpType, EditorRegistry, FileEditor, Version, write_with_backup};
use crate::domain::git::command::GitCommandRunner;
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

pub fn run(args: ReleaseArgs) -> Result<()> {
    let resolved_files = resolve_file_paths(&args.files);

    if !args.no_root {
        switch_to_git_root()?;
    }

    let state = validate_git_state(&args)?;
    let registry = EditorRegistry::default_with_editors();
    let config_files = resolve_config_files(&registry, &resolved_files)?;

    show_release_plan(&registry, &config_files, &state)?;

    if args.dry_run {
        show_operations_plan(&args, &config_files, &state)
    } else {
        execute_release_operations(&args, &registry, &config_files, &state)
    }
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
    let output = runner.execute(&["rev-parse", "--show-toplevel"], None)?;
    let root = output.trim();

    if !root.is_empty() {
        std::env::set_current_dir(root)
            .map_err(|e| anyhow::anyhow!("无法切换到 git 根目录: {} - {}", root, e))?;
    }

    Ok(())
}

fn validate_git_state(args: &ReleaseArgs) -> Result<GitState> {
    let runner = GitCommandRunner::new();

    let current_branch = runner.execute(&["branch", "--show-current"], None)?;
    let current_branch = current_branch.trim().to_string();

    if !args.force && current_branch != "master" {
        return Err(AppError::release("只能在 master 分支上执行 release").into());
    }

    let previous_tag = get_current_version(&runner);
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
        current_branch,
        new_tag,
        commit_message,
    })
}

fn get_current_version(runner: &GitCommandRunner) -> Option<String> {
    let output = runner
        .execute(&["describe", "--tags", "--match", "v*"], None)
        .ok()?;
    output.split('-').next().map(|s| s.to_string())
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

fn show_release_plan(
    registry: &EditorRegistry,
    config_files: &[String],
    state: &GitState,
) -> Result<()> {
    Output::header("修改计划");

    for file_path in config_files {
        let editor = registry.detect_editor(Path::new(file_path)).unwrap();
        print_file_diff(editor, &state.new_tag, file_path)?;
    }

    Ok(())
}

fn show_operations_plan(
    args: &ReleaseArgs,
    config_files: &[String],
    state: &GitState,
) -> Result<()> {
    Output::dry_run_header("将要执行的操作:");
    for file_path in config_files {
        print_lockfile_update_plan(file_path);
    }

    Output::message("git add <files>");
    Output::message(&format!("git commit -m \"{}\"", state.commit_message));
    Output::message(&format!("git tag {}", state.new_tag));

    print_push_plan(args.skip_push, &state.current_branch, &state.new_tag);

    Ok(())
}

fn execute_release_operations(
    args: &ReleaseArgs,
    registry: &EditorRegistry,
    config_files: &[String],
    state: &GitState,
) -> Result<()> {
    let runner = GitCommandRunner::new();

    for file_path in config_files {
        let editor = registry.detect_editor(Path::new(file_path)).unwrap();
        edit_version_in_file(editor, &state.new_tag, file_path)?;
        update_lockfile_after_edit(file_path)?;
        runner.execute_with_success(&["add", file_path], None)?;
    }

    let root = runner.execute(&["rev-parse", "--show-toplevel"], None)?;
    let root_path = Path::new(root.trim());

    runner.execute_streaming(&["diff", "--cached"], root_path)?;
    runner.execute_with_success(&["commit", "-m", &state.commit_message], None)?;
    runner.execute_with_success(&["tag", &state.new_tag], None)?;
    push_to_remotes(args.skip_push, &state.current_branch, &state.new_tag)?;

    Ok(())
}

fn print_file_diff(editor: &dyn FileEditor, tag: &str, config_file: &str) -> Result<()> {
    let (original, edited) = compute_edited_content(editor, tag, config_file)?;

    Output::message(config_file);

    let old_lines: Vec<&str> = original.lines().collect();
    let new_lines: Vec<&str> = edited.lines().collect();

    for (line_num, (old_line, new_line)) in (1..).zip(old_lines.iter().zip(new_lines.iter())) {
        if old_line != new_line {
            Output::detail(&format!("L{} -", line_num), old_line);
            Output::detail(&format!("L{} +", line_num), new_line);
        }
    }

    Ok(())
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

fn edit_version_in_file(editor: &dyn FileEditor, tag: &str, config_file: &str) -> Result<()> {
    let (_, edited) = compute_edited_content(editor, tag, config_file)?;
    write_with_backup(config_file, &edited)?;
    Ok(())
}

fn parent_dir(path: &Path) -> &Path {
    let parent = path.parent().unwrap_or(Path::new("."));
    if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    }
}

fn print_lockfile_update_plan(config_file: &str) {
    let dir = parent_dir(Path::new(config_file));

    if config_file.ends_with("Cargo.toml") && dir.join("Cargo.lock").exists() {
        Output::message("cargo update --package <name>");
    } else if config_file.ends_with("package.json") {
        if dir.join("package-lock.json").exists() {
            Output::message("npm install --package-lock-only");
        }
        if dir.join("pnpm-lock.yaml").exists() {
            Output::message("pnpm install --lockfile-only");
        }
    }
}

fn update_lockfile_after_edit(config_file: &str) -> Result<()> {
    if config_file.ends_with("Cargo.toml") {
        update_cargo_lock(config_file)?;
    } else if config_file.ends_with("package.json") {
        update_js_lockfile(config_file)?;
    }
    Ok(())
}

fn update_js_lockfile(package_json_path: &str) -> Result<()> {
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
        if pkg_dir.join(lock_name).exists() {
            return run_js_lockfile_update(pkg_dir, lock_name, cmd, args);
        }
    }

    if crate::utils::is_command_available("pnpm") {
        return run_js_lockfile_update(
            pkg_dir,
            "pnpm-lock.yaml",
            "pnpm",
            &["install", "--lockfile-only"],
        );
    }

    Ok(())
}

fn run_js_lockfile_update(
    pkg_dir: &Path,
    lock_name: &str,
    cmd: &str,
    args: &[&str],
) -> Result<()> {
    let lock_path = pkg_dir.join(lock_name);

    if is_gitignored(&lock_path) {
        Output::skip(&format!("{} 在 .gitignore 中，跳过更新", lock_name));
        return Ok(());
    }

    let cmd_str = format!("{} {}", cmd, args.join(" "));
    Output::cmd(&cmd_str);

    #[cfg(target_os = "windows")]
    let status = std::process::Command::new("cmd")
        .args(
            std::iter::once(&"/c")
                .chain(std::iter::once(&cmd))
                .chain(args.iter())
                .copied(),
        )
        .current_dir(pkg_dir)
        .status()
        .map_err(|e| anyhow::anyhow!("无法执行 {}: {}", cmd_str, e))?;
    #[cfg(not(target_os = "windows"))]
    let status = std::process::Command::new(cmd)
        .args(args)
        .current_dir(pkg_dir)
        .status()
        .map_err(|e| anyhow::anyhow!("无法执行 {}: {}", cmd_str, e))?;

    if !status.success() {
        Output::error(&format!("{} 执行失败", cmd_str));
    }

    if lock_path.exists() {
        let path_str = lock_path.to_string_lossy().replace('\\', "/");
        GitCommandRunner::new().execute_with_success(&["add", &path_str], None)?;
    }
    Ok(())
}

fn update_cargo_lock(cargo_toml_path: &str) -> Result<()> {
    let dir = parent_dir(Path::new(cargo_toml_path));
    let lock_path = dir.join("Cargo.lock");

    if !lock_path.exists() {
        return Ok(());
    }

    if is_gitignored(&lock_path) {
        Output::skip("Cargo.lock 在 .gitignore 中，跳过更新");
        return Ok(());
    }

    let pkg_name = read_cargo_package_name(cargo_toml_path)?;
    let runner = GitCommandRunner::new();

    Output::cmd(&format!("cargo update --package {}", pkg_name));
    let status = std::process::Command::new("cargo")
        .args(["update", "--package", &pkg_name])
        .current_dir(dir)
        .status()
        .map_err(|e| anyhow::anyhow!("无法执行 cargo update: {}", e))?;

    if !status.success() {
        return Err(
            AppError::release(format!("cargo update --package {} 执行失败", pkg_name)).into(),
        );
    }

    let path_str = lock_path.to_string_lossy().replace('\\', "/");
    runner.execute_with_success(&["add", &path_str], None)?;
    Ok(())
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

fn get_remotes(runner: &GitCommandRunner) -> Vec<String> {
    let Ok(output) = runner.execute(&["remote"], None) else {
        return Vec::new();
    };
    output
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn print_push_plan(skip_push: bool, current_branch: &str, new_tag: &str) {
    if skip_push {
        return;
    }

    let runner = GitCommandRunner::new();
    for remote in get_remotes(&runner) {
        Output::message(&format!("git push {} {}", remote, new_tag));
        Output::message(&format!("git push {} {}", remote, current_branch));
    }
}

fn push_to_remotes(skip_push: bool, current_branch: &str, new_tag: &str) -> Result<()> {
    if skip_push {
        return Ok(());
    }

    let runner = GitCommandRunner::new();
    for remote in get_remotes(&runner) {
        if let Err(e) = runner.execute_with_success(&["push", &remote, new_tag], None) {
            Output::warning(&format!("推送标签失败: {}", e));
        }
        if let Err(e) = runner.execute_with_success(&["push", &remote, current_branch], None) {
            Output::warning(&format!("推送分支失败: {}", e));
        }
    }

    Ok(())
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
