use super::{GitCommandRunner, Result};
use crate::model::git::{Branch, GitContext, Remote, Tag};
use std::path::Path;

pub fn collect_context(repo_path: &Path) -> Result<GitContext> {
    let runner = GitCommandRunner::new();

    let root = runner.execute(&["rev-parse", "--show-toplevel"], Some(repo_path))?;
    let root = std::path::PathBuf::from(root);

    let current_branch = runner.get_current_branch(&root)?;
    let remotes = collect_remotes(&runner, &root)?;
    let branches = collect_branches(&runner, &root)?;
    let tags = collect_tags(&runner, &root)?;
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
    let names = runner.get_remote_list(root)?;
    let mut remotes = Vec::new();
    for name in &names {
        if let Ok(url) = runner.execute(&["remote", "get-url", name], Some(root)) {
            remotes.push(Remote {
                name: name.to_string(),
                url,
            });
        }
    }
    Ok(remotes)
}

fn collect_branches(runner: &GitCommandRunner, root: &Path) -> Result<Vec<Branch>> {
    let output = runner.execute(&["branch", "-vv", "--all"], Some(root))?;
    let mut branches = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let is_current = line.starts_with("* ");
        let line = line.trim_start_matches("* ").trim_start_matches("  ");

        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        let name = parts.first().unwrap_or(&line).to_string();
        let name = name.trim_start_matches("remotes/").to_string();

        let is_remote = line.contains("remotes/");
        let info = parts.get(1).unwrap_or(&"");

        branches.push(Branch {
            name,
            is_current: is_current && !is_remote,
            is_remote,
            tracking_branch: extract_tracking_branch(info),
        });
    }

    Ok(branches)
}

fn collect_tags(runner: &GitCommandRunner, root: &Path) -> Result<Vec<Tag>> {
    let output = runner.execute(
        &["for-each-ref", "--format=%(refname:short)", "refs/tags"],
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

fn extract_tracking_branch(info: &str) -> Option<String> {
    if let Some(start) = info.find('[')
        && let Some(end) = info.find(']')
    {
        let inner = &info[start + 1..end];
        if inner.contains(':') {
            Some(inner.to_string())
        } else {
            None
        }
    } else {
        None
    }
}
