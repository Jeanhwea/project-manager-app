use crate::domain::config::ConfigDir;
use crate::domain::git::command::GitCommandRunner;
use crate::utils::output::Output;
use std::path::Path;

pub struct RemoteIssue {
    pub current_name: String,
    pub expected_name: String,
    pub host: String,
}

pub fn resolve_remote_name(host: &str) -> Option<String> {
    let config = ConfigDir::load_config();
    for rule in &config.remote.rules {
        if rule.hosts.iter().any(|h| h == host) {
            return Some(rule.name.clone());
        }
    }
    None
}

pub fn diagnose_remote_names(repo_path: &Path) -> Vec<RemoteIssue> {
    let runner = GitCommandRunner::new();
    let mut issues = Vec::new();

    let Ok(remotes) = runner.get_all_remotes(repo_path) else {
        return issues;
    };

    if remotes.len() == 1 && remotes[0].name == "origin" {
        return issues;
    }

    for remote in &remotes {
        if let Some(host) = remote.extract_host()
            && let Some(expected_name) = resolve_remote_name(&host)
            && expected_name != remote.name
        {
            issues.push(RemoteIssue {
                current_name: remote.name.clone(),
                expected_name,
                host,
            });
        }
    }

    issues
}

pub fn fix_remote_names(repo_path: &Path, issues: &[RemoteIssue], dry_run: bool) -> usize {
    let runner = GitCommandRunner::new();
    let mut fixed = 0;

    for issue in issues {
        if dry_run {
            Output::skip(&format!(
                "重命名 remote: {} -> {} (主机: {})",
                issue.current_name, issue.expected_name, issue.host
            ));
            fixed += 1;
            continue;
        }

        let Ok(remotes) = runner.get_remote_list(repo_path) else {
            continue;
        };
        if remotes.iter().any(|r| *r == issue.expected_name) {
            Output::warning(&format!(
                "目标 remote 名称 {} 已存在，跳过",
                issue.expected_name
            ));
            continue;
        }

        match runner.execute_with_success(
            &["remote", "rename", &issue.current_name, &issue.expected_name],
            Some(repo_path),
        ) {
            Ok(()) => {
                Output::success(&format!(
                    "已重命名 remote: {} -> {}",
                    issue.current_name, issue.expected_name
                ));
                fixed += 1;
            }
            Err(e) => Output::error(&format!("无法重命名 remote: {}", e)),
        }
    }

    fixed
}
