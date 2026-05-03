use super::{Command, CommandError, CommandResult};
use crate::domain::editor::{EditorRegistry, FileEditor, write_with_backup};
use crate::domain::git::command::GitCommandRunner;
use crate::utils::output::{ItemColor, Output};
use crate::utils::path::canonicalize_path;
use anyhow::{Context, Result};
use regex::Regex;
use std::path::{Path, PathBuf};

/// Release command arguments
#[derive(Debug)]
pub struct ReleaseArgs {
    /// Bump type: major, minor, or patch
    pub bump_type: BumpType,
    /// Files to update version (auto-detect if empty)
    pub files: Vec<String>,
    /// Stay in current directory instead of switching to git root
    pub no_root: bool,
    /// Force release even if not on master branch
    pub force: bool,
    /// Skip pushing tags and branches to remotes
    pub skip_push: bool,
    /// Dry run: show what would be changed without making any modifications
    pub dry_run: bool,
    /// Custom commit message (tag name will be prepended automatically)
    pub message: Option<String>,
    /// Pre-release suffix (e.g. "alpha", "rc.1" -> v1.0.0-alpha)
    pub pre_release: Option<String>,
}

/// Bump type enumeration
#[derive(Debug, Clone, Copy)]
pub enum BumpType {
    Major,
    Minor,
    Patch,
}

impl BumpType {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            BumpType::Major => "major",
            BumpType::Minor => "minor",
            BumpType::Patch => "patch",
        }
    }
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
        // Convert domain errors to command errors
        match execute_release(args) {
            Ok(()) => Ok(()),
            Err(e) => {
                // Convert anyhow errors to CommandError
                Err(CommandError::ExecutionFailed(format!("{}", e)))
            }
        }
    }
}

/// Main release execution function
fn execute_release(args: ReleaseArgs) -> Result<()> {
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
fn switch_to_git_root() -> Result<()> {
    let runner = GitCommandRunner::new();
    let output = runner.execute(&["rev-parse", "--show-toplevel"])?;
    let root = output.trim();

    if !root.is_empty() {
        std::env::set_current_dir(root)
            .with_context(|| format!("无法切换到 git 根目录: {}", root))?;
    }

    Ok(())
}

/// Validate git state and prepare release
fn validate_git_state(args: &ReleaseArgs) -> Result<GitState> {
    let runner = GitCommandRunner::new();

    let current_branch = runner.execute(&["branch", "--show-current"])?;
    let current_branch = current_branch.trim().to_string();

    if !args.force && current_branch != "master" {
        anyhow::bail!("只能在 master 分支上执行 release");
    }

    let current_tag = get_current_version(&runner).unwrap_or_else(|| "v0.0.0".to_string());

    let rev_current_tag = runner.execute(&["rev-parse", &current_tag])?;
    let rev_head = runner.execute(&["rev-parse", "HEAD"])?;
    if rev_current_tag.trim() == rev_head.trim() {
        anyhow::bail!("当前 HEAD 已被标记为 {}", current_tag);
    }

    let version = parse_version_from_tag(&current_tag).unwrap_or_default();
    let new_version = version.bump(args.bump_type.as_str());
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
    let tag = tag.trim();
    let tag = tag.strip_prefix('v').unwrap_or(tag);
    let parts: Vec<&str> = tag.split('.').collect();

    if parts.len() != 3 {
        return None;
    }

    Some(Version {
        major: parts[0].parse().ok()?,
        minor: parts[1].parse().ok()?,
        patch: parts[2].parse().ok()?,
    })
}

/// Resolve configuration files
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

/// Detect configuration files automatically
fn detect_config_files(registry: &EditorRegistry) -> Result<Vec<ConfigFileEntry>> {
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
        anyhow::bail!("未检测到可编辑的配置文件");
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
) -> Result<()> {
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
) -> Result<()> {
    let runner = GitCommandRunner::new();

    // Update files and lockfiles
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
) -> Result<()> {
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
) -> Result<(String, String)> {
    let version = tag.trim_start_matches('v');
    let content = std::fs::read_to_string(config_file)
        .with_context(|| format!("无法读取 {}", config_file))?;

    let edited = registry.edit_version(editor, &content, version)?;

    Ok((content, edited))
}

/// Edit version in a file
fn edit_version_in_file(
    registry: &EditorRegistry,
    editor: &dyn FileEditor,
    tag: &str,
    config_file: &str,
) -> Result<()> {
    let (_original, edited) = compute_edited_content(registry, editor, tag, config_file)?;
    write_with_backup(config_file, &edited).map_err(|e| anyhow::anyhow!("{}", e))?;
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
fn update_lockfile_after_edit(config_file: &str) -> Result<()> {
    if config_file.ends_with("Cargo.toml") {
        update_cargo_lock(config_file)?;
    } else if config_file.ends_with("package.json") {
        update_npm_lock(config_file)?;
        update_pnpm_lock(config_file)?;
    }
    Ok(())
}

/// Update Cargo.lock file
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
        Output::skip("Cargo.lock 在 .gitignore 中，跳过更新");
        return Ok(());
    }

    let pkg_name = read_cargo_package_name(cargo_toml_path)?;
    let runner = GitCommandRunner::new();

    let status = std::process::Command::new("cargo")
        .args(["update", "--package", &pkg_name])
        .current_dir(dir)
        .status()
        .with_context(|| "无法执行 cargo update")?;

    if !status.success() {
        anyhow::bail!("cargo update --package {} 执行失败", pkg_name);
    }

    let lock_str = lock_path.to_string_lossy().to_string();
    runner.execute_with_success(&["add", &lock_str])?;
    Ok(())
}

/// Update npm lockfile
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
        Output::skip("package-lock.json 在 .gitignore 中，跳过更新");
        return Ok(());
    }

    let status = std::process::Command::new("npm")
        .args(["install", "--package-lock-only"])
        .current_dir(&lock_dir)
        .status()
        .with_context(|| "无法执行 npm install")?;

    if !status.success() {
        Output::error("npm install --package-lock-only 执行失败");
    }

    let lock_str = lock_path.to_string_lossy().to_string();
    let runner = GitCommandRunner::new();
    runner.execute_with_success(&["add", &lock_str])?;
    Ok(())
}

/// Update pnpm lockfile
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
        Output::skip("pnpm-lock.yaml 在 .gitignore 中，跳过更新");
        return Ok(());
    }

    let status = std::process::Command::new("pnpm")
        .args(["install", "--lockfile-only"])
        .current_dir(&lock_dir)
        .status()
        .with_context(|| "无法执行 pnpm install")?;

    if !status.success() {
        Output::error("pnpm install --lockfile-only 执行失败");
    }

    let lock_str = lock_path.to_string_lossy().to_string();
    let runner = GitCommandRunner::new();
    runner.execute_with_success(&["add", &lock_str])?;
    Ok(())
}

/// Find directory containing lockfile
/// Searches from pkg_dir up to git root
fn find_lock_dir(pkg_dir: &Path, lock_file: &str) -> Option<PathBuf> {
    // Check pkg_dir itself
    if pkg_dir.join(lock_file).exists() {
        return Some(pkg_dir.to_path_buf());
    }

    // Find git root and check there
    let runner = GitCommandRunner::new();
    if let Ok(git_root) = runner.execute_in_dir(&["rev-parse", "--show-toplevel"], pkg_dir) {
        let git_root = git_root.trim();
        let git_root_path = Path::new(git_root);
        if git_root_path.join(lock_file).exists() {
            return Some(git_root_path.to_path_buf());
        }
    }

    None
}

/// Check if file is gitignored
fn is_gitignored(file_path: &Path) -> bool {
    let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let Some(parent) = file_path.parent() else {
        return false;
    };

    let runner = GitCommandRunner::new();
    let output = runner.execute_quiet_in_dir(&["check-ignore", file_name], parent);

    match output {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

/// Read package name from Cargo.toml
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

/// Print push plan for dry run
fn print_push_plan(skip_push: bool, current_branch: &str, new_tag: &str) {
    if skip_push {
        return;
    }

    let runner = GitCommandRunner::new();
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
fn push_to_remotes(skip_push: bool, current_branch: &str, new_tag: &str) -> Result<()> {
    if skip_push {
        return Ok(());
    }

    let runner = GitCommandRunner::new();
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

/// Version structure for parsing and bumping
#[derive(Debug, Clone, Default)]
struct Version {
    major: u32,
    minor: u32,
    patch: u32,
}

impl Version {
    fn bump(&self, bump_type: &str) -> Self {
        match bump_type {
            "major" => Version {
                major: self.major + 1,
                minor: 0,
                patch: 0,
            },
            "minor" => Version {
                major: self.major,
                minor: self.minor + 1,
                patch: 0,
            },
            _ => Version {
                major: self.major,
                minor: self.minor,
                patch: self.patch + 1,
            },
        }
    }

    fn to_tag(&self) -> String {
        format!("v{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_bump_type_as_str() {
        assert_eq!(BumpType::Major.as_str(), "major");
        assert_eq!(BumpType::Minor.as_str(), "minor");
        assert_eq!(BumpType::Patch.as_str(), "patch");
    }

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
        let bumped = version.bump("major");
        assert_eq!(bumped.major, 2);
        assert_eq!(bumped.minor, 0);
        assert_eq!(bumped.patch, 0);

        // Test minor bump
        let bumped = version.bump("minor");
        assert_eq!(bumped.major, 1);
        assert_eq!(bumped.minor, 3);
        assert_eq!(bumped.patch, 0);

        // Test patch bump
        let bumped = version.bump("patch");
        assert_eq!(bumped.major, 1);
        assert_eq!(bumped.minor, 2);
        assert_eq!(bumped.patch, 4);

        // Test default bump (should be patch)
        let bumped = version.bump("unknown");
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
        // Test that ReleaseArgs can be created
        let args = ReleaseArgs {
            bump_type: BumpType::Patch,
            files: vec!["Cargo.toml".to_string()],
            no_root: false,
            force: false,
            skip_push: true,
            dry_run: true,
            message: Some("Test release".to_string()),
            pre_release: None,
        };

        assert_eq!(args.bump_type.as_str(), "patch");
        assert_eq!(args.files.len(), 1);
        assert!(!args.no_root);
        assert!(!args.force);
        assert!(args.skip_push);
        assert!(args.dry_run);
        assert_eq!(args.message.unwrap(), "Test release");
        assert!(args.pre_release.is_none());
    }
}
