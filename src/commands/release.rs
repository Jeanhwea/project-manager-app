use super::{Command, CommandError, CommandResult};
use crate::domain::context::AppContext;
use crate::domain::editor::{BumpType, EditorRegistry, FileEditor, Version, write_with_backup};
use crate::domain::git::command::GitCommandRunner;
use crate::utils::output::{ItemColor, Output};
use crate::utils::path::canonicalize_path;
use regex::Regex;
use std::path::Path;

/// Release command arguments
#[derive(Debug, clap::Args)]
pub struct ReleaseArgs {
    /// Bump type: major, minor, patch
    #[arg(
        value_enum,
        default_value = "patch",
        help = "Bump type: major, minor, patch"
    )]
    pub bump_type: crate::cli::BumpType,
    /// Files to update version (auto-detect if not specified)
    #[arg(help = "Files to update version (auto-detect if not specified)")]
    pub files: Vec<String>,
    /// Stay in current directory instead of switching to git root
    #[arg(
        long,
        short = 'n',
        default_value = "false",
        help = "Stay in current directory instead of switching to git root"
    )]
    pub no_root: bool,
    /// Force release even if not on master branch
    #[arg(
        long,
        short,
        default_value = "false",
        help = "Force release even if not on master branch"
    )]
    pub force: bool,
    /// Skip pushing tags and branches to remotes
    #[arg(
        long,
        default_value = "false",
        help = "Skip pushing tags and branches to remotes"
    )]
    pub skip_push: bool,
    /// Dry run: show what would be changed without making any modifications
    #[arg(
        long,
        default_value = "false",
        help = "Dry run: show what would be changed without making any modifications"
    )]
    pub dry_run: bool,
    /// Custom commit message (tag name will be prepended automatically)
    #[arg(
        long,
        short = 'm',
        help = "Custom commit message (tag name will be prepended automatically)"
    )]
    pub message: Option<String>,
    /// Pre-release suffix (e.g. "alpha", "rc.1" -> v1.0.0-alpha)
    #[arg(
        long,
        help = "Pre-release suffix (e.g. \"alpha\", \"rc.1\" -> v1.0.0-alpha)"
    )]
    pub pre_release: Option<String>,
}

/// Git state for release operations
struct GitState {
    current_branch: String,
    new_tag: String,
    commit_message: String,
}

/// Configuration file entry
type ConfigFileEntry = (String, std::sync::Arc<dyn FileEditor>);

/// Configuration file candidates for auto-detection
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

/// Release command
pub struct ReleaseCommand;

impl Command for ReleaseCommand {
    type Args = ReleaseArgs;

    fn execute(args: Self::Args) -> CommandResult {
        execute_release(args)
    }
}

/// Main release execution function
fn execute_release(args: ReleaseArgs) -> CommandResult {
    let resolved_files = resolve_file_paths(&args.files);

    if !args.no_root {
        switch_to_git_root()?;
    }

    let state = validate_git_state(&args)?;

    let registry = EditorRegistry::default_with_editors();

    let config_files = resolve_config_files(&registry, &resolved_files)?;

    if args.dry_run {
        execute_dry_run(&args, &registry, &config_files, &state)
    } else {
        execute_release_operations(&args, &registry, &config_files, &state)
    }
}

/// Resolve file paths to absolute paths
fn resolve_file_paths(files: &[String]) -> Vec<String> {
    files
        .iter()
        .map(|f| {
            if Path::new(f).is_absolute() {
                f.clone()
            } else {
                canonicalize_path(f)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| f.clone())
            }
        })
        .collect()
}

/// Switch to git root directory
fn switch_to_git_root() -> CommandResult {
    let runner = AppContext::global().git_runner();
    let output = runner.execute(&["rev-parse", "--show-toplevel"])?;
    let root = output.trim();

    if !root.is_empty() {
        std::env::set_current_dir(root).map_err(|e| {
            CommandError::ExecutionFailed(format!("无法切换到 git 根目录: {} - {}", root, e))
        })?;
    }

    Ok(())
}

/// Validate git state and prepare release
fn validate_git_state(args: &ReleaseArgs) -> Result<GitState, CommandError> {
    let runner = AppContext::global().git_runner();

    let current_branch = runner.execute(&["branch", "--show-current"])?;
    let current_branch = current_branch.trim().to_string();

    if !args.force && current_branch != "master" {
        return Err(CommandError::Validation(
            "只能在 master 分支上执行 release".to_string(),
        ));
    }

    let current_tag = get_current_version(runner).unwrap_or_else(|| "v0.0.0".to_string());

    let rev_current_tag = runner.execute(&["rev-parse", &current_tag])?;
    let rev_head = runner.execute(&["rev-parse", "HEAD"])?;
    if rev_current_tag.trim() == rev_head.trim() {
        return Err(CommandError::Validation(format!(
            "当前 HEAD 已被标记为 {}",
            current_tag
        )));
    }

    let version = parse_version_from_tag(&current_tag).unwrap_or_default();
    let new_version = version.bump(&to_domain_bump_type(args.bump_type.clone()));
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

/// Get current version from git tags
fn get_current_version(runner: &GitCommandRunner) -> Option<String> {
    let output = runner
        .execute(&["describe", "--tags", "--match", "v*"])
        .ok()?;
    // git describe may return "v1.0.0-3-g1234567" format, we only need the version part
    output.split('-').next().map(|s| s.to_string())
}

/// Parse version from tag string
fn parse_version_from_tag(tag: &str) -> Option<Version> {
    Version::from_tag(tag)
}

/// Convert release BumpType to domain BumpType
fn to_domain_bump_type(bump_type: crate::cli::BumpType) -> BumpType {
    match bump_type {
        crate::cli::BumpType::Major => BumpType::Major,
        crate::cli::BumpType::Minor => BumpType::Minor,
        crate::cli::BumpType::Patch => BumpType::Patch,
    }
}

/// Resolve configuration files
fn resolve_config_files(
    registry: &EditorRegistry,
    files: &[String],
) -> Result<Vec<ConfigFileEntry>, CommandError> {
    if files.is_empty() {
        return detect_config_files(registry);
    }

    files
        .iter()
        .map(|f| {
            let path = Path::new(f);
            let editor = registry
                .detect_editor(path)
                .ok_or_else(|| CommandError::Validation(format!("无法识别文件类型: {}", f)))?;
            Ok((f.clone(), editor))
        })
        .collect()
}

/// Detect configuration files automatically
fn detect_config_files(registry: &EditorRegistry) -> Result<Vec<ConfigFileEntry>, CommandError> {
    let mut result = Vec::new();

    for (pattern, is_dynamic) in CONFIG_FILE_CANDIDATES {
        if *is_dynamic && pattern.contains("{}") {
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
        return Err(CommandError::Validation(
            "未检测到可编辑的配置文件".to_string(),
        ));
    }

    Ok(result)
}

/// Expand glob pattern with dynamic parts
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

/// Execute dry run (show what would be changed)
fn execute_dry_run(
    args: &ReleaseArgs,
    registry: &EditorRegistry,
    config_files: &[ConfigFileEntry],
    state: &GitState,
) -> CommandResult {
    Output::dry_run_header("将要修改的文件:");
    for (file_path, editor) in config_files {
        print_file_diff(registry, editor.as_ref(), &state.new_tag, file_path)?;
    }

    Output::dry_run_header("将要执行的操作:");
    for (file_path, _editor) in config_files {
        print_lockfile_update_plan(file_path);
    }

    Output::message("git add <files>");
    Output::message(&format!("git commit -m \"{}\"", state.commit_message));
    Output::message(&format!("git tag {}", state.new_tag));

    print_push_plan(args.skip_push, &state.current_branch, &state.new_tag);

    Ok(())
}

/// Execute actual release operations
fn execute_release_operations(
    args: &ReleaseArgs,
    registry: &EditorRegistry,
    config_files: &[ConfigFileEntry],
    state: &GitState,
) -> CommandResult {
    let runner = AppContext::global().git_runner();

    for (file_path, editor) in config_files {
        edit_version_in_file(registry, editor.as_ref(), &state.new_tag, file_path)?;
        update_lockfile_after_edit(file_path)?;
        runner.execute_with_success(&["add", file_path])?;
    }

    // Show staged changes
    runner.execute_with_success(&["diff", "--cached"])?;

    // Commit changes
    runner.execute_with_success(&["commit", "-m", &state.commit_message])?;

    runner.execute_with_success(&["tag", &state.new_tag])?;

    push_to_remotes(args.skip_push, &state.current_branch, &state.new_tag)?;

    Ok(())
}

/// Print file diff for dry run
fn print_file_diff(
    registry: &EditorRegistry,
    editor: &dyn FileEditor,
    tag: &str,
    config_file: &str,
) -> CommandResult {
    let (original, edited) = compute_edited_content(registry, editor, tag, config_file)?;

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

/// Compute edited content for a file
fn compute_edited_content(
    registry: &EditorRegistry,
    editor: &dyn FileEditor,
    tag: &str,
    config_file: &str,
) -> Result<(String, String), CommandError> {
    let version = tag.trim_start_matches('v');
    let content = std::fs::read_to_string(config_file)
        .map_err(|e| CommandError::ExecutionFailed(format!("无法读取 {}: {}", config_file, e)))?;

    let edited = registry.edit_version(editor, &content, version)?;

    Ok((content, edited))
}

/// Edit version in a file
fn edit_version_in_file(
    registry: &EditorRegistry,
    editor: &dyn FileEditor,
    tag: &str,
    config_file: &str,
) -> CommandResult {
    let (_original, edited) = compute_edited_content(registry, editor, tag, config_file)?;
    write_with_backup(config_file, &edited)?;
    Ok(())
}

/// Print lockfile update plan for dry run
fn print_lockfile_update_plan(config_file: &str) {
    let parent = Path::new(config_file).parent().unwrap_or(Path::new("."));
    let dir = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };

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

/// Update lockfile after editing a configuration file
fn update_lockfile_after_edit(config_file: &str) -> CommandResult {
    if config_file.ends_with("Cargo.toml") {
        update_cargo_lock(config_file)?;
    } else if config_file.ends_with("package.json") {
        update_js_lockfile(config_file)?;
    }
    Ok(())
}

/// Detect and update JS lockfile based on actual package manager
fn update_js_lockfile(package_json_path: &str) -> CommandResult {
    let parent = Path::new(package_json_path)
        .parent()
        .unwrap_or(Path::new("."));
    let pkg_dir = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };

    // 检测实际使用的包管理器
    let pkg_manager = detect_package_manager(pkg_dir);

    match pkg_manager {
        PackageManager::Pnpm => update_pnpm_lock(package_json_path),
        PackageManager::Npm => update_npm_lock(package_json_path),
        PackageManager::Yarn => update_yarn_lock(package_json_path),
        PackageManager::None => Ok(()),
    }
}

/// Detected package manager type
enum PackageManager {
    Pnpm,
    Npm,
    Yarn,
    None,
}

/// Detect which package manager is used by checking lockfiles
fn is_pnpm_available() -> bool {
    // Windows 上需要使用 cmd /c 来执行 pnpm
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "pnpm", "--version"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("pnpm")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }
}

fn detect_package_manager(pkg_dir: &Path) -> PackageManager {
    // 只在 package.json 所在目录检测 lockfile
    // 优先检查 pnpm-lock.yaml
    if pkg_dir.join("pnpm-lock.yaml").exists() {
        return PackageManager::Pnpm;
    }
    // 然后检查 yarn.lock
    if pkg_dir.join("yarn.lock").exists() {
        return PackageManager::Yarn;
    }
    // 检查 package-lock.json，但优先使用 pnpm 命令
    if pkg_dir.join("package-lock.json").exists() {
        // 检查 pnpm 是否可用，优先使用 pnpm
        if is_pnpm_available() {
            return PackageManager::Pnpm;
        }
        return PackageManager::Npm;
    }
    // 没有 lockfile 时，检查 pnpm 是否可用
    if is_pnpm_available() {
        return PackageManager::Pnpm;
    }
    PackageManager::None
}

/// Update Cargo.lock file
fn update_cargo_lock(cargo_toml_path: &str) -> CommandResult {
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
        Output::skip("Cargo.lock 在 .gitignore 中，跳过更新");
        return Ok(());
    }

    let pkg_name = read_cargo_package_name(cargo_toml_path)?;
    let runner = AppContext::global().git_runner();

    Output::cmd(&format!("cargo update --package {}", pkg_name));
    let status = std::process::Command::new("cargo")
        .args(["update", "--package", &pkg_name])
        .current_dir(dir)
        .status()
        .map_err(|e| CommandError::ExecutionFailed(format!("无法执行 cargo update: {}", e)))?;

    if !status.success() {
        return Err(CommandError::ExecutionFailed(format!(
            "cargo update --package {} 执行失败",
            pkg_name
        )));
    }

    let lock_str = lock_path.to_string_lossy().to_string();
    runner.execute_with_success(&["add", &lock_str])?;
    Ok(())
}

/// Update npm lockfile
fn update_npm_lock(package_json_path: &str) -> CommandResult {
    let parent = Path::new(package_json_path)
        .parent()
        .unwrap_or(Path::new("."));
    let pkg_dir = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };

    let lock_path = pkg_dir.join("package-lock.json");

    if is_gitignored(&lock_path) {
        Output::skip("package-lock.json 在 .gitignore 中，跳过更新");
        return Ok(());
    }

    Output::cmd("npm install --package-lock-only");
    #[cfg(target_os = "windows")]
    let status = std::process::Command::new("cmd")
        .args(["/c", "npm", "install", "--package-lock-only"])
        .current_dir(pkg_dir)
        .status()
        .map_err(|e| CommandError::ExecutionFailed(format!("无法执行 npm install: {}", e)))?;

    #[cfg(not(target_os = "windows"))]
    let status = std::process::Command::new("npm")
        .args(["install", "--package-lock-only"])
        .current_dir(pkg_dir)
        .status()
        .map_err(|e| CommandError::ExecutionFailed(format!("无法执行 npm install: {}", e)))?;

    if !status.success() {
        Output::error("npm install --package-lock-only 执行失败");
    }

    if lock_path.exists() {
        let lock_str = lock_path.to_string_lossy().to_string();
        let runner = AppContext::global().git_runner();
        runner.execute_with_success(&["add", &lock_str])?;
    }
    Ok(())
}

fn update_pnpm_lock(package_json_path: &str) -> CommandResult {
    let parent = Path::new(package_json_path)
        .parent()
        .unwrap_or(Path::new("."));
    let pkg_dir = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };

    let lock_path = pkg_dir.join("pnpm-lock.yaml");

    if is_gitignored(&lock_path) {
        Output::skip("pnpm-lock.yaml 在 .gitignore 中，跳过更新");
        return Ok(());
    }

    Output::cmd("pnpm install --lockfile-only");
    #[cfg(target_os = "windows")]
    let status = std::process::Command::new("cmd")
        .args(["/c", "pnpm", "install", "--lockfile-only"])
        .current_dir(pkg_dir)
        .status()
        .map_err(|e| CommandError::ExecutionFailed(format!("无法执行 pnpm install: {}", e)))?;

    #[cfg(not(target_os = "windows"))]
    let status = std::process::Command::new("pnpm")
        .args(["install", "--lockfile-only"])
        .current_dir(pkg_dir)
        .status()
        .map_err(|e| CommandError::ExecutionFailed(format!("无法执行 pnpm install: {}", e)))?;

    if !status.success() {
        Output::error("pnpm install --lockfile-only 执行失败");
    }

    if lock_path.exists() {
        let lock_str = lock_path.to_string_lossy().to_string();
        let runner = AppContext::global().git_runner();
        runner.execute_with_success(&["add", &lock_str])?;
    }
    Ok(())
}

fn update_yarn_lock(package_json_path: &str) -> CommandResult {
    let parent = Path::new(package_json_path)
        .parent()
        .unwrap_or(Path::new("."));
    let pkg_dir = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };

    let lock_path = pkg_dir.join("yarn.lock");

    if is_gitignored(&lock_path) {
        Output::skip("yarn.lock 在 .gitignore 中，跳过更新");
        return Ok(());
    }

    Output::cmd("yarn install --mode update-lockfile");
    #[cfg(target_os = "windows")]
    let status = std::process::Command::new("cmd")
        .args(["/c", "yarn", "install", "--mode", "update-lockfile"])
        .current_dir(pkg_dir)
        .status()
        .map_err(|e| CommandError::ExecutionFailed(format!("无法执行 yarn install: {}", e)))?;

    #[cfg(not(target_os = "windows"))]
    let status = std::process::Command::new("yarn")
        .args(["install", "--mode", "update-lockfile"])
        .current_dir(pkg_dir)
        .status()
        .map_err(|e| CommandError::ExecutionFailed(format!("无法执行 yarn install: {}", e)))?;

    if !status.success() {
        Output::error("yarn install --mode update-lockfile 执行失败");
    }

    if lock_path.exists() {
        let lock_str = lock_path.to_string_lossy().to_string();
        let runner = AppContext::global().git_runner();
        runner.execute_with_success(&["add", &lock_str])?;
    }
    Ok(())
}

fn is_gitignored(file_path: &Path) -> bool {
    let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let Some(parent) = file_path.parent() else {
        return false;
    };

    let runner = AppContext::global().git_runner();
    let output = runner.execute_quiet_in_dir(&["check-ignore", file_name], parent);

    match output {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

/// Read package name from Cargo.toml
fn read_cargo_package_name(cargo_toml_path: &str) -> Result<String, CommandError> {
    let content = std::fs::read_to_string(cargo_toml_path).map_err(|e| {
        CommandError::ExecutionFailed(format!("无法读取 {}: {}", cargo_toml_path, e))
    })?;
    let re = Regex::new(r#"name\s*=\s*"([^"]*)""#)
        .map_err(|e| CommandError::ExecutionFailed(format!("正则表达式错误: {}", e)))?;
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
    Err(CommandError::ExecutionFailed(format!(
        "未在 {} 中找到 [package] name",
        cargo_toml_path
    )))
}

/// Print push plan for dry run
fn print_push_plan(skip_push: bool, current_branch: &str, new_tag: &str) {
    if skip_push {
        return;
    }

    let runner = AppContext::global().git_runner();
    let remotes = match runner.execute(&["remote"]) {
        Ok(output) => output
            .lines()
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>(),
        Err(_) => return,
    };

    for remote in remotes {
        Output::message(&format!("git push {} {}", remote, new_tag));
        Output::message(&format!("git push {} {}", remote, current_branch));
    }
}

/// Push to remotes
fn push_to_remotes(skip_push: bool, current_branch: &str, new_tag: &str) -> CommandResult {
    if skip_push {
        return Ok(());
    }

    let runner = AppContext::global().git_runner();
    let remotes = match runner.execute(&["remote"]) {
        Ok(output) => output
            .lines()
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>(),
        Err(_) => return Ok(()),
    };

    for remote in remotes {
        if let Err(e) = runner.execute_with_success(&["push", &remote, new_tag]) {
            Output::warning(&format!("推送标签失败: {}", e));
        }
        if let Err(e) = runner.execute_with_success(&["push", &remote, current_branch]) {
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
    fn test_version_parsing() {
        // Test valid version parsing
        let version = parse_version_from_tag("v1.2.3").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);

        // Test version without v prefix
        let version = parse_version_from_tag("2.3.4").unwrap();
        assert_eq!(version.major, 2);
        assert_eq!(version.minor, 3);
        assert_eq!(version.patch, 4);

        // Test invalid version parsing
        assert!(parse_version_from_tag("invalid").is_none());
        assert!(parse_version_from_tag("1.2").is_none());
        assert!(parse_version_from_tag("v1.2").is_none());
    }

    #[test]
    fn test_version_bumping() {
        let version = Version {
            major: 1,
            minor: 2,
            patch: 3,
        };

        // Test major bump
        let bumped = version.bump(&BumpType::Major);
        assert_eq!(bumped.major, 2);
        assert_eq!(bumped.minor, 0);
        assert_eq!(bumped.patch, 0);

        // Test minor bump
        let bumped = version.bump(&BumpType::Minor);
        assert_eq!(bumped.major, 1);
        assert_eq!(bumped.minor, 3);
        assert_eq!(bumped.patch, 0);

        // Test patch bump
        let bumped = version.bump(&BumpType::Patch);
        assert_eq!(bumped.major, 1);
        assert_eq!(bumped.minor, 2);
        assert_eq!(bumped.patch, 4);
    }

    #[test]
    fn test_version_to_tag() {
        let version = Version {
            major: 1,
            minor: 2,
            patch: 3,
        };
        assert_eq!(version.to_tag(), "v1.2.3");
    }

    #[test]
    fn test_resolve_file_paths() {
        // Test with absolute path
        let files = vec!["/absolute/path".to_string()];
        let resolved = resolve_file_paths(&files);
        assert_eq!(resolved, vec!["/absolute/path".to_string()]);

        // Test with relative path (will be canonicalized in real scenario)
        let files = vec!["relative/path".to_string()];
        let resolved = resolve_file_paths(&files);
        // In test, canonicalize_path might fail, so it returns the original
        assert_eq!(resolved.len(), 1);
    }

    #[test]
    fn test_expand_glob_pattern() {
        // Create a temporary directory with subdirectories
        let temp_dir = tempdir().unwrap();
        let dir1 = temp_dir.path().join("dir1");
        let dir2 = temp_dir.path().join("dir2");
        std::fs::create_dir(&dir1).unwrap();
        std::fs::create_dir(&dir2).unwrap();

        // Change to temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Test pattern expansion
        let pattern = "{}/package.json";
        let expanded = expand_glob_pattern(pattern);

        // Should find dir1 and dir2
        assert!(expanded.contains(&"dir1/package.json".to_string()));
        assert!(expanded.contains(&"dir2/package.json".to_string()));

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_read_cargo_package_name() {
        let temp_dir = tempdir().unwrap();
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");

        // Create a test Cargo.toml file
        let content = r#"[package]
name = "test-package"
version = "0.1.0"

[dependencies]
serde = "1.0""#;

        std::fs::write(&cargo_toml_path, content).unwrap();

        let package_name = read_cargo_package_name(&cargo_toml_path.to_string_lossy()).unwrap();
        assert_eq!(package_name, "test-package");
    }

    #[test]
    fn test_read_cargo_package_name_not_found() {
        let temp_dir = tempdir().unwrap();
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");

        // Create a Cargo.toml file without package name
        let content = r#"[dependencies]
serde = "1.0""#;

        std::fs::write(&cargo_toml_path, content).unwrap();

        let result = read_cargo_package_name(&cargo_toml_path.to_string_lossy());
        assert!(result.is_err());
    }

    #[test]
    fn test_release_args_structure() {
        let args = ReleaseArgs {
            bump_type: crate::cli::BumpType::Patch,
            files: vec!["Cargo.toml".to_string()],
            no_root: false,
            force: false,
            skip_push: true,
            dry_run: true,
            message: Some("Test release".to_string()),
            pre_release: None,
        };

        assert_eq!(args.files.len(), 1);
        assert!(!args.no_root);
        assert!(!args.force);
        assert!(args.skip_push);
        assert!(args.dry_run);
        assert_eq!(args.message.unwrap(), "Test release");
        assert!(args.pre_release.is_none());
    }
}
