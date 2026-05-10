use crate::domain::config::ConfigDir;
use crate::domain::git::command::GitCommandRunner;
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
