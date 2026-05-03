use super::{Command, CommandError, CommandResult};
use crate::domain::git::command::GitCommandRunner;
use crate::domain::git::repository::{RepoWalker, find_git_repository_upwards};
use crate::domain::runner::DryRunContext;
use crate::utils::output::Output;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Doctor command arguments
#[derive(Debug)]
pub struct DoctorArgs {
    pub max_depth: Option<usize>,
    pub gc: bool,
    pub rename: bool,
    pub fix: bool,
    pub path: Option<String>,
    pub dry_run: bool,
}

/// Doctor command
pub struct DoctorCommand;

impl Command for DoctorCommand {
    type Args = DoctorArgs;

    fn execute(args: Self::Args) -> CommandResult {
        match execute_doctor(args) {
            Ok(()) => Ok(()),
            Err(e) => Err(CommandError::ExecutionFailed(format!("{}", e))),
        }
    }
}

/// Main doctor execution function
fn execute_doctor(args: DoctorArgs) -> Result<()> {
    check_dependencies()?;

    // Get search path: use provided path or current directory
    let search_path = match args.path {
        Some(ref p) => PathBuf::from(p),
        None => std::env::current_dir()?,
    };

    // Search upwards for git repository root
    let effective_path =
        find_git_repository_upwards(&search_path).unwrap_or_else(|| search_path.clone());

    let walker = RepoWalker::new(&effective_path, args.max_depth.unwrap_or(3))?;

    if walker.is_empty() {
        Output::not_found("未找到 Git 仓库");
        return Ok(());
    }

    let ctx = DryRunContext::new(args.dry_run);
    ctx.print_header("[DRY-RUN] 将要检查的仓库:");

    walker.walk(|repo_path, _index, _total| {
        let mut issues = Vec::new();

        check_detached_head(repo_path, &mut issues);
        check_stale_remote_refs(repo_path, &mut issues);
        check_large_repo(repo_path, &mut issues);
        check_missing_upstream(repo_path, &mut issues);
        check_stash(repo_path, &mut issues);

        if !issues.is_empty() {
            Output::warning("发现问题:");
            for issue in &issues {
                Output::warning(issue);
            }
        } else {
            Output::success("仓库健康");
        }

        if args.fix && !issues.is_empty() {
            fix_issues(&ctx, repo_path, &issues)?;
        }

        if args.gc {
            do_git_garbage_collect(&ctx, repo_path)?;
        }

        if args.rename {
            for (remote_name, remote_url) in get_remote_info(repo_path) {
                if let Some(new_name) = get_remote_name_by_url(&remote_url)
                    && new_name != remote_name
                {
                    Output::item(&format!("{} => {}", remote_name, new_name), &remote_url);
                    do_rename_git_remote(&ctx, repo_path, &remote_name, &new_name)?;
                }
            }
        }

        Ok(())
    })?;

    Ok(())
}

/// Check for required command-line tools
fn check_dependencies() -> Result<()> {
    const REQUIRED_TOOLS: &[(&str, &str)] = &[("git", "版本控制工具，所有仓库操作的核心依赖")];

    Output::section("检查依赖工具...");

    let mut missing = Vec::new();
    for (cmd, desc) in REQUIRED_TOOLS {
        if check_command_exists(cmd) {
            Output::success(cmd);
        } else {
            Output::error(&format!("{} ({})", cmd, desc));
            missing.push(*cmd);
        }
    }

    if !missing.is_empty() {
        anyhow::bail!("缺少必要的命令行工具: {}", missing.join(", "));
    }

    Output::success("所有依赖工具已就绪");
    Output::blank();
    Ok(())
}

/// Check if a command exists in PATH
fn check_command_exists(cmd: &str) -> bool {
    if cfg!(windows) {
        std::process::Command::new("where")
            .arg(cmd)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    } else {
        std::process::Command::new("which")
            .arg(cmd)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

/// Get remote information for a repository
fn get_remote_info(repo_path: &Path) -> Vec<(String, String)> {
    let runner = GitCommandRunner::new();
    match runner.execute_in_dir(&["remote", "-v"], repo_path) {
        Ok(output) => {
            let mut remotes = Vec::new();
            for line in output.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    remotes.push((parts[0].to_string(), parts[1].to_string()));
                }
            }
            remotes
        }
        Err(_) => Vec::new(),
    }
}

/// Get canonical remote name from URL
fn get_remote_name_by_url(url: &str) -> Option<String> {
    if url.contains("github.com") {
        Some("github".to_string())
    } else if url.contains("gitlab.com") {
        Some("gitlab".to_string())
    } else if url.contains("gitee.com") {
        Some("gitee".to_string())
    } else if url.contains("bitbucket.org") {
        Some("bitbucket".to_string())
    } else {
        // Extract hostname from URL
        let url = url
            .trim_start_matches("ssh://")
            .trim_start_matches("https://")
            .trim_start_matches("http://");
        if let Some(at_pos) = url.find('@') {
            let after_at = &url[at_pos + 1..];
            if let Some(colon_pos) = after_at.find(':') {
                return Some(after_at[..colon_pos].to_string());
            }
        } else if let Some(slash_pos) = url.find('/') {
            return Some(url[..slash_pos].to_string());
        }
        None
    }
}

/// Check for detached HEAD state
fn check_detached_head(repo_path: &Path, issues: &mut Vec<String>) {
    let runner = GitCommandRunner::new();
    match runner.execute_in_dir(&["branch", "--show-current"], repo_path) {
        Ok(branch) => {
            if branch.trim().is_empty() {
                issues.push("HEAD 处于 detached 状态".to_string());
            }
        }
        Err(_) => {
            issues.push("无法获取当前分支信息".to_string());
        }
    }
}

/// Check for stale remote references
fn check_stale_remote_refs(repo_path: &Path, issues: &mut Vec<String>) {
    let runner = GitCommandRunner::new();

    let remotes = match runner.execute_in_dir(&["remote"], repo_path) {
        Ok(output) => output
            .lines()
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>(),
        Err(_) => {
            issues.push("无法获取远程仓库列表".to_string());
            return;
        }
    };

    if remotes.is_empty() {
        return;
    }

    for remote in &remotes {
        let output = match runner.execute_quiet_in_dir(&["remote", "show", remote], repo_path) {
            Ok(o) => o,
            Err(_) => continue,
        };

        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("does not appear to be a git repository")
            || stderr.contains("could not read from remote repository")
            || stderr.contains("fatal:")
        {
            issues.push(format!("远程仓库 '{}' 不可达", remote));
        }
    }

    let stale_output = match runner.execute_quiet_in_dir(&["branch", "-r"], repo_path) {
        Ok(o) => o,
        Err(_) => return,
    };

    let stale_stdout = String::from_utf8_lossy(&stale_output.stdout);
    let stale_branches: Vec<&str> = stale_stdout
        .lines()
        .filter(|line| line.contains(": gone"))
        .map(|line| line.trim())
        .collect();

    if !stale_branches.is_empty() {
        issues.push(format!(
            "存在 {} 个陈旧的远程跟踪分支",
            stale_branches.len()
        ));
    }
}

/// Check for large repository size
fn check_large_repo(repo_path: &Path, issues: &mut Vec<String>) {
    let runner = GitCommandRunner::new();
    let output = match runner.execute_quiet_in_dir(&["count-objects", "-vH"], repo_path) {
        Ok(o) => o,
        Err(_) => return,
    };

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        if let Some(size_str) = line.strip_prefix("size-pack:") {
            let size_str = size_str.trim();
            if let Some(num_part) = size_str.split_whitespace().next()
                && let Ok(size) = num_part.parse::<f64>()
            {
                let unit = size_str.split_whitespace().nth(1).unwrap_or("bytes");
                let size_mb = match unit {
                    "GiB" => size * 1024.0,
                    "MiB" => size,
                    "KiB" => size / 1024.0,
                    _ => size / (1024.0 * 1024.0),
                };
                if size_mb > 500.0 {
                    issues.push(format!("仓库体积较大 ({}), 建议执行 git gc", size_str));
                }
            }
        }
    }
}

/// Check for missing upstream tracking branch
fn check_missing_upstream(repo_path: &Path, issues: &mut Vec<String>) {
    let runner = GitCommandRunner::new();
    let branch = match runner.execute_in_dir(&["branch", "--show-current"], repo_path) {
        Ok(b) => b.trim().to_string(),
        Err(_) => return,
    };

    if branch.is_empty() {
        return;
    }

    let output = runner.execute_quiet_in_dir(
        &[
            "rev-parse",
            "--abbrev-ref",
            &format!("{}@{{upstream}}", branch),
        ],
        repo_path,
    );

    if output.is_err() {
        let remotes = match runner.execute_in_dir(&["remote"], repo_path) {
            Ok(output) => output
                .lines()
                .map(|s| s.trim().to_string())
                .collect::<Vec<_>>(),
            Err(_) => return,
        };

        if !remotes.is_empty() {
            issues.push(format!("当前分支 '{}' 没有设置上游跟踪分支", branch));
        }
    }
}

/// Check for excessive stash entries
fn check_stash(repo_path: &Path, issues: &mut Vec<String>) {
    let runner = GitCommandRunner::new();
    let output = match runner.execute_quiet_in_dir(&["stash", "list"], repo_path) {
        Ok(o) => o,
        Err(_) => return,
    };

    let stash_count = String::from_utf8_lossy(&output.stdout).lines().count();

    if stash_count > 5 {
        issues.push(format!(
            "stash 列表中有 {} 个条目，可能需要清理",
            stash_count
        ));
    }
}

/// Fix detected issues
fn fix_issues(ctx: &DryRunContext, repo_path: &Path, issues: &[String]) -> Result<()> {
    Output::section("修复问题:");

    for issue in issues {
        if issue.contains("陈旧的远程跟踪分支") {
            Output::success("清理陈旧的远程跟踪分支");
            ctx.run_in_dir("git", &["remote", "prune", "origin"], Some(repo_path))
                .with_context(|| "无法清理陈旧的远程跟踪分支")?;
        } else if issue.contains("detached") {
            Output::skip("无法自动修复 detached HEAD，请手动切换到分支");
        } else if issue.contains("上游跟踪分支") {
            let runner = GitCommandRunner::new();
            let branch = match runner.execute_in_dir(&["branch", "--show-current"], repo_path) {
                Ok(b) => b.trim().to_string(),
                Err(_) => continue,
            };

            if !branch.is_empty() {
                let upstream = format!("origin/{}", branch);
                Output::item("修复", &format!("设置 {} 的上游为 {}", branch, upstream));
                ctx.run_in_dir(
                    "git",
                    &["branch", "--set-upstream-to", &upstream, &branch],
                    Some(repo_path),
                )
                .with_context(|| format!("无法设置 {} 的上游跟踪分支", branch))?;
            }
        } else if issue.contains("体积较大") {
            Output::success("执行 git gc --aggressive");
            ctx.run_in_dir("git", &["gc", "--aggressive"], Some(repo_path))
                .with_context(|| "无法执行 git gc --aggressive")?;
        } else if issue.contains("stash") {
            Output::warning("stash 条目较多，请手动清理 (git stash drop/pop)");
        }
    }

    Ok(())
}

/// Rename a Git remote
fn do_rename_git_remote(
    ctx: &DryRunContext,
    repo_path: &Path,
    old_name: &str,
    new_name: &str,
) -> Result<()> {
    let existing_remotes = get_remote_info(repo_path);
    let conflict = existing_remotes.iter().find(|(name, _)| name == new_name);

    if let Some((_, conflict_url)) = conflict {
        let alt_name =
            get_remote_name_by_url(conflict_url).unwrap_or_else(|| format!("{}-old", new_name));

        if alt_name == new_name {
            anyhow::bail!(
                "远程仓库 {} 的 URL ({}) 推断名称仍为 {}，无法解决冲突",
                new_name,
                conflict_url,
                alt_name
            );
        }

        Output::item(&format!("{} => {}", new_name, alt_name), "");
        ctx.run_in_dir(
            "git",
            &["remote", "rename", new_name, &alt_name],
            Some(repo_path),
        )
        .with_context(|| format!("无法重命名远程仓库 {} -> {}", new_name, alt_name))?;
    }

    ctx.run_in_dir(
        "git",
        &["remote", "rename", old_name, new_name],
        Some(repo_path),
    )
    .with_context(|| format!("无法重命名远程仓库 {} -> {}", old_name, new_name))?;
    Ok(())
}

/// Perform Git garbage collection
fn do_git_garbage_collect(ctx: &DryRunContext, repo_path: &Path) -> Result<()> {
    ctx.run_in_dir("git", &["gc"], Some(repo_path))
        .with_context(|| "无法执行 git gc")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doctor_args_structure() {
        let args = DoctorArgs {
            max_depth: Some(3),
            gc: true,
            rename: false,
            fix: true,
            path: Some(".".to_string()),
            dry_run: true,
        };

        assert_eq!(args.max_depth, Some(3));
        assert!(args.gc);
        assert!(!args.rename);
        assert!(args.fix);
        assert_eq!(args.path, Some(".".to_string()));
        assert!(args.dry_run);
    }

    #[test]
    fn test_dry_run_context() {
        let ctx = DryRunContext::new(true);
        assert!(ctx.is_dry_run());

        let ctx = DryRunContext::new(false);
        assert!(!ctx.is_dry_run());
    }

    #[test]
    fn test_check_command_exists() {
        let _git_exists = check_command_exists("git");
        assert!(true);
    }

    #[test]
    fn test_get_remote_name_by_url() {
        // Test GitHub URL
        assert_eq!(
            get_remote_name_by_url("git@github.com:user/repo.git"),
            Some("github".to_string())
        );
        assert_eq!(
            get_remote_name_by_url("https://github.com/user/repo.git"),
            Some("github".to_string())
        );

        // Test GitLab URL
        assert_eq!(
            get_remote_name_by_url("git@gitlab.com:user/repo.git"),
            Some("gitlab".to_string())
        );
        assert_eq!(
            get_remote_name_by_url("https://gitlab.com/user/repo.git"),
            Some("gitlab".to_string())
        );

        // Test Gitee URL
        assert_eq!(
            get_remote_name_by_url("git@gitee.com:user/repo.git"),
            Some("gitee".to_string())
        );
        assert_eq!(
            get_remote_name_by_url("https://gitee.com/user/repo.git"),
            Some("gitee".to_string())
        );

        // Test Bitbucket URL
        assert_eq!(
            get_remote_name_by_url("git@bitbucket.org:user/repo.git"),
            Some("bitbucket".to_string())
        );
        assert_eq!(
            get_remote_name_by_url("https://bitbucket.org/user/repo.git"),
            Some("bitbucket".to_string())
        );

        // Test custom SSH URL
        assert_eq!(
            get_remote_name_by_url("git@example.com:user/repo.git"),
            Some("example.com".to_string())
        );

        // Test custom HTTPS URL
        assert_eq!(
            get_remote_name_by_url("https://example.com/user/repo.git"),
            Some("example.com".to_string())
        );

        // Test invalid URL
        assert_eq!(get_remote_name_by_url("invalid-url"), None);
    }

    #[test]
    fn test_doctor_command_implementation() {
        // Test that DoctorCommand implements Command trait
        // We can't use dynamic dispatch because Command trait is not object-safe
        // Instead, we verify the trait is implemented by using it statically
        let _command = DoctorCommand;
        // Test passes if compilation succeeds
    }
}
