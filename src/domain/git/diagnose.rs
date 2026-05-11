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
    StaleRefs {
        remote: String,
    },
    LargeRepo {
        size_bytes: u64,
    },
    RemoteNameMismatch {
        current: String,
        expected: String,
        host: String,
    },
}

impl Diagnosis {
    pub fn display_message(&self) -> String {
        match self {
            Diagnosis::DetachedHead => "HEAD 处于 detached 状态".to_string(),
            Diagnosis::NoRemote => "没有配置远程仓库".to_string(),
            Diagnosis::NoRemoteTrackingBranch => "没有远程跟踪分支".to_string(),
            Diagnosis::SingleLocalBranch => "只有一个本地分支".to_string(),
            Diagnosis::StashExists => "存在 stash 条目".to_string(),
            Diagnosis::StaleRefs { remote } => format!("远程仓库 {} 的引用已陈旧", remote),
            Diagnosis::LargeRepo { size_bytes } => {
                format!("仓库大小较大: {}", format_bytes(*size_bytes))
            }
            Diagnosis::RemoteNameMismatch {
                current,
                expected,
                host,
            } => {
                format!(
                    "remote 名称不匹配: {} -> {} (主机: {})",
                    current, expected, host
                )
            }
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.2} GiB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MiB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KiB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

const LARGE_REPO_THRESHOLD_BYTES: u64 = 100 * 1024 * 1024;

pub fn diagnose_repo(repo_path: &Path) -> Result<Vec<Diagnosis>, GitError> {
    let mut issues = Vec::new();
    let runner = GitCommandRunner::new();

    let output = runner.execute(&["symbolic-ref", "HEAD"], Some(repo_path))?;
    if output.trim().is_empty() {
        issues.push(Diagnosis::DetachedHead);
    }

    let output = runner.execute(&["remote"], Some(repo_path))?;
    if output.trim().is_empty() {
        issues.push(Diagnosis::NoRemote);
    }

    if let Ok(output) = runner.execute(&["branch", "-r"], Some(repo_path)) {
        let remote_branches: Vec<&str> = output.lines().collect();
        if remote_branches.is_empty() {
            issues.push(Diagnosis::NoRemoteTrackingBranch);
        }
    }

    if let Ok(output) = runner.execute(&["branch", "--list"], Some(repo_path)) {
        let local_branches: Vec<&str> = output.lines().collect();
        if local_branches.len() == 1 {
            issues.push(Diagnosis::SingleLocalBranch);
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
        issues.push(Diagnosis::StashExists);
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
                issues.push(Diagnosis::StaleRefs {
                    remote: remote.to_string(),
                });
            }
        }
    }

    if let Ok(output) = runner.execute(&["count-objects", "-v"], Some(repo_path)) {
        for line in output.lines() {
            if let Some(size_str) = line.strip_prefix("size-pack:") {
                if let Some(size_num) = size_str.trim().split_whitespace().next() {
                    if let Ok(size_bytes) = size_num.parse::<u64>() {
                        if size_bytes > LARGE_REPO_THRESHOLD_BYTES {
                            issues.push(Diagnosis::LargeRepo { size_bytes });
                        }
                    }
                }
            }
        }
    }

    for remote_issue in diagnose_remote_names(repo_path) {
        issues.push(Diagnosis::RemoteNameMismatch {
            current: remote_issue.current_name,
            expected: remote_issue.expected_name,
            host: remote_issue.host,
        });
    }

    Ok(issues)
}
