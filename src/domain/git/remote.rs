use crate::domain::config::ConfigDir;
use crate::domain::git::command::GitCommandRunner;
use crate::utils::output::Output;
use std::path::Path;

pub fn extract_host_from_url(remote_url: &str) -> Option<String> {
    if remote_url.starts_with("git@") {
        remote_url
            .split(':')
            .next()
            .and_then(|s| s.strip_prefix("git@"))
            .map(String::from)
    } else if let Ok(url) = url::Url::parse(remote_url) {
        url.host_str().map(String::from)
    } else {
        None
    }
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

pub struct RemoteIssue {
    pub current_name: String,
    pub expected_name: String,
    pub host: String,
}

pub fn diagnose_remote_names(repo_path: &Path) -> Vec<RemoteIssue> {
    let runner = GitCommandRunner::new();
    let mut issues = Vec::new();

    let Ok(remotes) = runner.get_remote_list(repo_path) else {
        return issues;
    };

    for remote_name in &remotes {
        if remotes.len() == 1 && remote_name == "origin" {
            continue;
        }
        let Ok(url) = runner.execute(&["remote", "get-url", remote_name], Some(repo_path)) else {
            continue;
        };
        let Some(host) = extract_host_from_url(&url) else {
            continue;
        };
        let Some(expected_name) = resolve_remote_name(&host) else {
            continue;
        };
        if expected_name != *remote_name {
            issues.push(RemoteIssue {
                current_name: remote_name.clone(),
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
            &[
                "remote",
                "rename",
                &issue.current_name,
                &issue.expected_name,
            ],
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
