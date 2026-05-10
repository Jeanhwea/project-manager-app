use super::GitCommandRunner;
use super::remote::diagnose_remote_names;
use std::path::Path;

pub fn diagnose_repo(repo_path: &Path) -> Vec<String> {
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
