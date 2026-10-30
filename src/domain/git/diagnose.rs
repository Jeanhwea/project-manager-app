//! 仓库健康检查：把 git 查询中发现的可疑状态归一成 `Diagnosis` 列表。
//! 这里只做只读判断，是否修复由 `commands::doctor` 决定；单条查询失败按「没查到」处理而非中断。

use super::GitCommandRunner;
use super::remote::diagnose_remote_names;
use crate::domain::git::GitError;
use std::path::Path;

/// doctor 能识别的仓库异常项，每个变体对应一类提示或修复建议
#[derive(Debug, Clone)]
pub enum Diagnosis {
    DetachedHead,
    NoRemote,
    NoRemoteTrackingBranch,
    SingleLocalBranch,
    StashExists,
    StaleRefs { remote: String },
    LargeRepo,
    RemoteNameMismatch { current: String, expected: String },
}

/// count-objects 的 size-pack 超过该值即判定为大仓库
const LARGE_REPO_THRESHOLD_BYTES: u64 = 100 * 1024 * 1024;

/// 检查单个仓库并返回发现的所有问题
pub fn diagnose_repo(repo_path: &Path) -> Result<Vec<Diagnosis>, GitError> {
    let mut issues = Vec::new();
    let runner = GitCommandRunner::new();

    // symbolic-ref 在 HEAD 指向分支时输出 ref 名，detached HEAD 下则取不到内容
    let output = runner.run_local(&["symbolic-ref", "HEAD"], Some(repo_path))?;
    if output.trim().is_empty() {
        issues.push(Diagnosis::DetachedHead);
    }

    let output = runner.run_local(&["remote"], Some(repo_path))?;
    if output.trim().is_empty() {
        issues.push(Diagnosis::NoRemote);
    }

    // 一条远程跟踪分支都没有时，pull/push 大概率配错或从未推送过
    if let Ok(output) = runner.run_local(&["branch", "-r"], Some(repo_path)) {
        let remote_branches: Vec<&str> = output.lines().collect();
        if remote_branches.is_empty() {
            issues.push(Diagnosis::NoRemoteTrackingBranch);
        }
    }

    // 只有一条本地分支通常意味着还没建立分支策略，仅作提醒
    if let Ok(output) = runner.run_local(&["branch", "--list"], Some(repo_path)) {
        let local_branches: Vec<&str> = output.lines().collect();
        if local_branches.len() == 1 {
            issues.push(Diagnosis::SingleLocalBranch);
        }
    }

    // refs/stash 下只要有记录，就说明还有没落地的暂存改动
    if let Ok(output) = runner.run_local(
        &[
            "for-each-ref",
            "--sort=-creatordate",
            "--format=%(creatordate:iso)",
            "refs/stash",
        ],
        Some(repo_path),
    ) && !output.trim().is_empty()
    {
        issues.push(Diagnosis::StashExists);
    }

    // remote show 会真的连一次远端，靠输出里的 (stale) 标记发现已被删除的跟踪分支
    if let Ok(output) = runner.run_local(&["remote", "show"], Some(repo_path)) {
        for remote in output.lines() {
            let remote = remote.trim();
            if remote.is_empty() {
                continue;
            }
            if let Ok(remote_output) =
                runner.run_local(&["remote", "show", remote], Some(repo_path))
                && remote_output.contains("(stale)")
            {
                issues.push(Diagnosis::StaleRefs {
                    remote: remote.to_string(),
                });
            }
        }
    }

    // size-pack 是打包后的体积，-v 才给出这一行
    if let Ok(output) = runner.run_local(&["count-objects", "-v"], Some(repo_path)) {
        for line in output.lines() {
            if let Some(size_str) = line.strip_prefix("size-pack:")
                && let Some(size_num) = size_str.split_whitespace().next()
                && let Ok(size_bytes) = size_num.parse::<u64>()
                && size_bytes > LARGE_REPO_THRESHOLD_BYTES
            {
                issues.push(Diagnosis::LargeRepo);
            }
        }
    }

    for remote_issue in diagnose_remote_names(repo_path) {
        issues.push(Diagnosis::RemoteNameMismatch {
            current: remote_issue.current_name,
            expected: remote_issue.expected_name,
        });
    }

    Ok(issues)
}
