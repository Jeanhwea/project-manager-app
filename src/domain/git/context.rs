use super::{GitCommandRunner, Result};
use crate::model::git::{Branch, GitContext, Remote, Tag};
use std::path::Path;

pub fn collect_context(repo_path: &Path) -> Result<GitContext> {
    let runner = GitCommandRunner::new();
    collect_context_with_runner(&runner, repo_path)
}

pub fn collect_context_with_runner(
    runner: &GitCommandRunner,
    repo_path: &Path,
) -> Result<GitContext> {
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
            if remote_name.ends_with("/HEAD") {
                continue;
            }
            (remote_name.to_string(), true)
        } else if let Some(local_name) = refname.strip_prefix("refs/heads/") {
            (local_name.to_string(), false)
        } else {
            continue;
        };

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
