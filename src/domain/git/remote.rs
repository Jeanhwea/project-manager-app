//! git remote 的命名规约：按 remote url 的主机名匹配配置里的 `remote.rules`，
//! 检查是否有 remote 该改名（doctor 会据此给出修正建议）。

use super::GitCommandRunner;
use super::context::collect_remotes;
use crate::domain::config::ConfigManager;

/// 一处 remote 命名与约定不一致
pub struct RemoteIssue {
    pub current_name: String,
    pub expected_name: String,
}

/// 按主机名查出约定使用的 remote 名；没有命中任何 rule 时返回 None
pub fn resolve_remote_name(host: &str) -> Option<String> {
    let config = ConfigManager::load_config();
    for rule in &config.remote.rules {
        if rule.hosts.iter().any(|h| h == host) {
            return Some(rule.name.clone());
        }
    }
    None
}

/// 列出仓库中所有命名不符合约定的 remote
pub fn diagnose_remote_names(repo_path: &std::path::Path) -> Vec<RemoteIssue> {
    let runner = GitCommandRunner::new();
    let mut issues = Vec::new();

    let Ok(remotes) = collect_remotes(&runner, repo_path) else {
        return issues;
    };

    // 只有一个 remote 且已叫 origin 是最常见的情况，直接放行以免误报
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
            });
        }
    }

    issues
}
