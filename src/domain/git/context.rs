//! 采集仓库的只读状态快照，填充 [`crate::model::git::GitContext`]。
//! 全部走本地查询、不触网，因此可以安全地在 plan 阶段反复调用。

use super::{GitCommandRunner, Result};
use crate::model::git::{Branch, GitContext, Remote, Tag};
use std::path::Path;

/// 便捷入口：自带 runner 采集一次上下文
pub fn collect_context(repo_path: &Path) -> Result<GitContext> {
    let runner = GitCommandRunner::new();
    collect_context_with_runner(&runner, repo_path)
}

/// 允许调用方复用已有 runner（例如已经做过前置查询的场景）
pub fn collect_context_with_runner(
    runner: &GitCommandRunner,
    repo_path: &Path,
) -> Result<GitContext> {
    // 归一化到仓库根目录再查询：传入的 repo_path 可能是仓库内的任意子目录
    let root = runner.run_local(&["rev-parse", "--show-toplevel"], Some(repo_path))?;
    let root = std::path::PathBuf::from(root);

    let current_branch = runner.current_branch(&root)?;
    let remotes = collect_remotes(runner, &root)?;
    let branches = collect_branches(runner, &root)?;
    let tags = collect_tags(runner, &root)?;
    let has_uncommitted_changes = runner.has_uncommitted_changes(&root)?;

    Ok(GitContext {
        current_branch,
        remotes,
        branches,
        tags,
        has_uncommitted_changes,
    })
}

/// 列出所有 remote 及其 url；取不到 url 的 remote 会被静默丢弃
pub fn collect_remotes(runner: &GitCommandRunner, root: &Path) -> Result<Vec<Remote>> {
    let names = runner.remote_names(root)?;
    let mut remotes = Vec::new();
    for name in &names {
        if let Ok(url) = runner.run_local(&["remote", "get-url", name], Some(root)) {
            remotes.push(Remote {
                name: name.to_string(),
                url,
            });
        }
    }
    Ok(remotes)
}

fn collect_branches(runner: &GitCommandRunner, root: &Path) -> Result<Vec<Branch>> {
    // 用制表符（%09）分隔三个字段：分支名允许含空格，只有制表符才能安全切分
    let format = "%(refname)%09%(HEAD)%09%(upstream:short)";
    let output = runner.run_local(
        &[
            "for-each-ref",
            &format!("--format={}", format),
            "refs/heads",
            "refs/remotes",
        ],
        Some(root),
    )?;

    let mut branches = Vec::new();

    for line in output.lines() {
        if line.is_empty() {
            continue;
        }

        let mut parts = line.splitn(3, '\t');
        let refname = parts.next().unwrap_or("");
        let head_marker = parts.next().unwrap_or(" ");
        let upstream = parts.next().unwrap_or("");

        let (name, is_remote) = if let Some(remote_name) = refname.strip_prefix("refs/remotes/") {
            // <remote>/HEAD 是 git 自动生成的指向默认分支的符号 ref，不是真正的分支
            if remote_name.ends_with("/HEAD") {
                continue;
            }
            (remote_name.to_string(), true)
        } else if let Some(local_name) = refname.strip_prefix("refs/heads/") {
            (local_name.to_string(), false)
        } else {
            continue;
        };

        // %(HEAD) 只会标记 refs/heads 下的当前分支，远程 ref 不可能带 *
        let is_current = !is_remote && head_marker == "*";
        let tracking_branch = if !upstream.is_empty() && !is_remote {
            Some(upstream.to_string())
        } else {
            None
        };

        branches.push(Branch {
            name,
            is_current,
            is_remote,
            tracking_branch,
        });
    }

    Ok(branches)
}

fn collect_tags(runner: &GitCommandRunner, root: &Path) -> Result<Vec<Tag>> {
    // creatordate 倒序让最新 tag 排在首位，上层可以直接取第一个作为当前版本
    let output = runner.run_local(
        &[
            "for-each-ref",
            "--format=%(refname:short)",
            "--sort=-creatordate",
            "refs/tags",
        ],
        Some(root),
    )?;

    let mut tags = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        tags.push(Tag {
            name: line.to_string(),
        });
    }

    Ok(tags)
}
