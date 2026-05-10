use crate::domain::config::ConfigDir;
use crate::domain::git::GitCommandRunner;
use crate::model::git::Remote;

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

pub fn diagnose_remote_names(repo_path: &std::path::Path) -> Vec<RemoteIssue> {
    let runner = GitCommandRunner::new();
    let mut issues = Vec::new();

    let Ok(remotes) = collect_remotes(&runner, repo_path) else {
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

fn collect_remotes(
    runner: &GitCommandRunner,
    repo_path: &std::path::Path,
) -> anyhow::Result<Vec<Remote>> {
    let names = runner.get_remote_list(repo_path)?;
    let mut remotes = Vec::new();
    for name in &names {
        if let Ok(url) = runner.execute(&["remote", "get-url", name], Some(repo_path)) {
            let fetch_url = runner
                .execute(&["remote", "get-url", "--push", name], Some(repo_path))
                .ok();
            let fetch_url = fetch_url.filter(|u| *u != url);
            remotes.push(Remote {
                name: name.to_string(),
                url,
                fetch_url,
            });
        }
    }
    Ok(remotes)
}
