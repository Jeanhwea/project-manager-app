use super::GitCommandRunner;
use super::remote::diagnose_remote_names;
use crate::domain::git::GitError;
use std::path::Path;

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

const LARGE_REPO_THRESHOLD_BYTES: u64 = 100 * 1024 * 1024;

pub fn diagnose_repo(repo_path: &Path) -> Result<Vec<Diagnosis>, GitError> {
    let mut issues = Vec::new();
    let runner = GitCommandRunner::new();

    let output = runner.run_local(&["symbolic-ref", "HEAD"], Some(repo_path))?;
    if output.trim().is_empty() {
        issues.push(Diagnosis::DetachedHead);
    }

    let output = runner.run_local(&["remote"], Some(repo_path))?;
    if output.trim().is_empty() {
        issues.push(Diagnosis::NoRemote);
    }

    if let Ok(output) = runner.run_local(&["branch", "-r"], Some(repo_path)) {
        let remote_branches: Vec<&str> = output.lines().collect();
        if remote_branches.is_empty() {
            issues.push(Diagnosis::NoRemoteTrackingBranch);
        }
    }

    if let Ok(output) = runner.run_local(&["branch", "--list"], Some(repo_path)) {
        let local_branches: Vec<&str> = output.lines().collect();
        if local_branches.len() == 1 {
            issues.push(Diagnosis::SingleLocalBranch);
        }
    }

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
