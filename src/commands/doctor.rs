use crate::commands::{RepoPathArgs, init_repo_walker};
use crate::domain::AppError;
use crate::domain::git::command::GitCommandRunner;
use crate::domain::git::remote::{diagnose_remote_names, fix_remote_names};
use crate::utils::output::Output;
use std::path::Path;

#[derive(Debug, clap::Args)]
pub struct DoctorArgs {
    #[command(flatten)]
    pub repo_path: RepoPathArgs,
    #[arg(
        long,
        short,
        default_value = "false",
        help = "Automatically fix detected issues"
    )]
    pub fix: bool,
    #[arg(
        long,
        default_value = "false",
        help = "Dry run: show what would be fixed"
    )]
    pub dry_run: bool,
}

pub fn run(args: DoctorArgs) -> anyhow::Result<()> {
    let Some(walker) = init_repo_walker(&args.repo_path)? else {
        return Ok(());
    };

    if args.fix {
        check_prerequisites()?;
    }

    let mut total_issues = 0;
    let mut total_fixed = 0;

    for repo_info in walker.repositories() {
        let repo_path = &repo_info.path;
        let issues = diagnose_repo(repo_path);

        if issues.is_empty() {
            Output::success(&format!("{}: 健康", repo_path.display()));
            continue;
        }

        total_issues += issues.len();
        Output::warning(&format!("{}: {} 个问题", repo_path.display(), issues.len()));

        for issue in &issues {
            Output::detail("问题", issue);
        }

        if args.fix {
            let fixed = fix_issues(repo_path, &issues, args.dry_run)?;
            total_fixed += fixed;
        }
    }

    Output::header("诊断汇总");
    Output::item("检查仓库", &walker.total().to_string());
    Output::item("发现问题", &total_issues.to_string());
    if args.fix {
        Output::item("已修复", &total_fixed.to_string());
    }

    Ok(())
}

fn check_prerequisites() -> anyhow::Result<()> {
    let tools = ["git"];
    let missing: Vec<&str> = tools
        .iter()
        .filter(|tool| !crate::utils::is_command_available(tool))
        .copied()
        .collect();

    if !missing.is_empty() {
        return Err(AppError::command_not_available(&missing.join(", ")).into());
    }

    Ok(())
}

fn diagnose_repo(repo_path: &Path) -> Vec<String> {
    let mut issues = Vec::new();
    let runner = GitCommandRunner::new();

    if let Ok(output) = runner.execute(&["symbolic-ref", "HEAD"], Some(repo_path))
        && output.trim().is_empty()
    {
        issues.push("HEAD 处于 detached 状态".to_string());
    }

    if let Ok(output) = runner.execute(&["remote"], Some(repo_path))
        && output.trim().is_empty()
    {
        issues.push("没有配置远程仓库".to_string());
    }

    if let Ok(output) = runner.execute(&["branch", "-r"], Some(repo_path)) {
        let remote_branches: Vec<&str> = output.lines().collect();
        if remote_branches.is_empty() {
            issues.push("没有远程跟踪分支".to_string());
        }
    }

    if let Ok(output) = runner.execute(&["branch", "--list"], Some(repo_path)) {
        let local_branches: Vec<&str> = output.lines().collect();
        if local_branches.len() == 1 {
            issues.push("只有一个本地分支".to_string());
        }
    }

    if let Ok(output) = runner.execute(
        &[
            "for-each-ref",
            "--sort=-creatordate",
            "--format=%(creatordate:iso)",
            "refs/stash",
        ],
        Some(repo_path),
    ) && !output.trim().is_empty()
    {
        issues.push("存在 stash 条目".to_string());
    }

    if let Ok(output) = runner.execute(&["remote", "show"], Some(repo_path)) {
        for remote in output.lines() {
            let remote = remote.trim();
            if remote.is_empty() {
                continue;
            }
            if let Ok(remote_output) =
                runner.execute(&["remote", "show", remote], Some(repo_path))
                && remote_output.contains("(stale)")
            {
                issues.push(format!("远程仓库 {} 的引用已陈旧", remote));
            }
        }
    }

    if let Ok(output) = runner.execute(&["count-objects", "-vH"], Some(repo_path)) {
        for line in output.lines() {
            if let Some(size_str) = line.strip_prefix("size-pack:")
                && let Some(size_num) = size_str.split_whitespace().next()
                && let Ok(size) = size_num.parse::<f64>()
                && size > 100.0
            {
                issues.push(format!("仓库大小较大: {}", size_str.trim()));
            }
        }
    }

    for remote_issue in diagnose_remote_names(repo_path) {
        issues.push(format!(
            "remote 名称不匹配: {} -> {} (主机: {})",
            remote_issue.current_name, remote_issue.expected_name, remote_issue.host
        ));
    }

    issues
}

fn fix_issues(repo_path: &Path, issues: &[String], dry_run: bool) -> anyhow::Result<usize> {
    let runner = GitCommandRunner::new();
    let mut fixed = 0;

    for issue in issues {
        if dry_run {
            Output::skip(&format!("修复: {}", issue));
            fixed += 1;
            continue;
        }

        if issue.contains("陈旧") {
            match runner.execute_with_success(&["remote", "prune", "origin"], Some(repo_path)) {
                Ok(()) => {
                    Output::success("已清理陈旧的远程跟踪分支");
                    fixed += 1;
                }
                Err(e) => Output::error(&format!("无法清理陈旧的远程跟踪分支: {}", e)),
            }
        } else if issue.contains("上游跟踪分支") || issue.contains("只有一个本地分支")
        {
            if let Ok(branch) = runner.get_current_branch(repo_path) {
                match runner.execute_with_success(
                    &["branch", "--set-upstream-to", &format!("origin/{}", branch)],
                    Some(repo_path),
                ) {
                    Ok(()) => {
                        Output::success(&format!("已设置 {} 的上游跟踪分支", branch));
                        fixed += 1;
                    }
                    Err(e) => {
                        Output::error(&format!("无法设置 {} 的上游跟踪分支: {}", branch, e))
                    }
                }
            }
        } else if issue.contains("仓库大小较大") {
            match runner.execute_with_success(&["gc", "--aggressive"], Some(repo_path)) {
                Ok(()) => {
                    Output::success("已执行 git gc --aggressive");
                    fixed += 1;
                }
                Err(e) => Output::error(&format!("无法执行 git gc --aggressive: {}", e)),
            }
        } else if issue.contains("stash") {
            Output::warning("stash 条目需要手动处理");
        }
    }

    let remote_issues = diagnose_remote_names(repo_path);
    fixed += fix_remote_names(repo_path, &remote_issues, dry_run);

    Ok(fixed)
}
