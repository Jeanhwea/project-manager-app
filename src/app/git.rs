use super::runner::CommandRunner;
use super::version::compare_versions;
use anyhow::{Context, Result};
use std::path::Path;

const REDINF_PATH_PREFIXES: &[&str] = &["red_8/", "redtool/", "red_base/", "teampuzzle/"];

pub fn get_rev_revision(ref_name: &str) -> Result<String> {
    let output = CommandRunner::run_quiet("git", &["rev-parse", ref_name])
        .with_context(|| format!("无法获取 {} 的 revision", ref_name))?;

    let revision = String::from_utf8(output.stdout).with_context(|| "无法解析 revision 输出")?;
    let revision = revision.trim();

    if revision.is_empty() {
        anyhow::bail!("{} 的 revision 为空", ref_name);
    }

    Ok(revision.to_string())
}

pub fn get_current_version() -> Option<String> {
    let output = CommandRunner::run_quiet("git", &["tag", "-l", "v*"]).ok()?;

    let tags = String::from_utf8(output.stdout).ok()?;
    let mut tags: Vec<&str> = tags.lines().collect();

    if tags.is_empty() {
        return None;
    }

    tags.sort_by(|a, b| compare_versions(a, b));

    tags.first().map(|s| s.to_string())
}

pub fn get_current_branch() -> Option<String> {
    let output = CommandRunner::run_quiet("git", &["branch", "--show-current"]).ok()?;

    let branch = String::from_utf8(output.stdout).ok()?;
    let branch = branch.trim();

    if branch.is_empty() {
        return None;
    }

    Some(branch.to_string())
}

pub fn get_remote_list() -> Option<Vec<String>> {
    let output = CommandRunner::run_quiet("git", &["remote"]).ok()?;

    let remotes = String::from_utf8(output.stdout).ok()?;
    let remotes: Vec<String> = remotes
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if remotes.is_empty() {
        return None;
    }

    Some(remotes)
}

pub fn list_cached_changes() -> Result<()> {
    CommandRunner::run_with_success("git", &["diff", "--cached"])
        .with_context(|| "无法列出缓存的更改")
        .map(|_| ())
}

pub fn clone(repo_url: &str, name: &str) -> Result<()> {
    CommandRunner::run_with_success("git", &["clone", repo_url, name])
        .with_context(|| format!("无法克隆仓库 {} 到 {}", repo_url, name))
        .map(|_| ())
}

pub fn add_file(file: &str) -> Result<()> {
    CommandRunner::run_with_success("git", &["add", file])
        .with_context(|| format!("无法添加文件 {}", file))
        .map(|_| ())
}

pub fn commit(message: &str) -> Result<()> {
    CommandRunner::run_with_success("git", &["commit", "-m", message])
        .with_context(|| format!("无法提交: {}", message))
        .map(|_| ())
}

pub fn create_tag(tag: &str) -> Result<()> {
    CommandRunner::run_with_success("git", &["tag", tag])
        .with_context(|| format!("无法创建标签 {}", tag))
        .map(|_| ())
}

pub fn push_tag(remote: &str, tag: &str) -> Result<()> {
    CommandRunner::run_with_success("git", &["push", remote, tag])
        .with_context(|| format!("无法推送标签 {} 到 {}", tag, remote))
        .map(|_| ())
}

pub fn push_branch(remote: &str, branch: &str) -> Result<()> {
    CommandRunner::run_with_success("git", &["push", remote, branch])
        .with_context(|| format!("无法推送分支 {} 到 {}", branch, remote))
        .map(|_| ())
}

pub fn get_remote_name(work_dir: &Path) -> Vec<String> {
    let output = match CommandRunner::run_quiet_in_dir("git", &["remote"], work_dir) {
        Ok(out) => out,
        Err(_) => return Vec::new(),
    };

    let remotes = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    remotes
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn get_remote_info(work_dir: &Path) -> Vec<(String, String)> {
    let remotes = get_remote_name(work_dir);
    if remotes.is_empty() {
        return Vec::new();
    }

    let mut remote_info = Vec::new();
    for remote in remotes {
        let output = match CommandRunner::run_quiet_in_dir(
            "git",
            &["remote", "get-url", &remote],
            work_dir,
        ) {
            Ok(out) => out,
            Err(_) => continue,
        };

        let url = match String::from_utf8(output.stdout) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let url = url.trim();
        if url.is_empty() {
            continue;
        }

        remote_info.push((remote, url.to_string()));
    }

    remote_info
}

pub fn parse_git_remote_url(url: &str) -> Option<(String, String, String)> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }

    let protocol = if url.starts_with("git@") {
        "git".to_string()
    } else if url.starts_with("ssh://git@") {
        "git".to_string()
    } else if url.starts_with("https://") {
        "https".to_string()
    } else if url.starts_with("http://") {
        "http".to_string()
    } else {
        return None;
    };

    let (url, separator) = if url.starts_with("git@") {
        (url.replace("git@", ""), ':')
    } else if url.starts_with("ssh://git@") {
        (url.replace("ssh://git@", ""), ':')
    } else if url.starts_with("https://") {
        (url.replace("https://", ""), '/')
    } else if url.starts_with("http://") {
        (url.replace("http://", ""), '/')
    } else {
        (url.to_string(), ':')
    };

    let parts: Vec<&str> = url.splitn(2, separator).collect();
    if parts.len() != 2 {
        return None;
    }

    let (host, path) = (parts[0].to_string(), parts[1].to_string());
    Some((protocol, host, path))
}

pub fn get_remote_name_by_url(url: &str) -> Option<String> {
    let (_, host, path) = parse_git_remote_url(url)?;

    let remote_name = if host == "github.com" || host == "githubfast.com" {
        "github".to_string()
    } else if host == "gitana.jeanhwea.io" {
        "gitana".to_string()
    } else if host == "gitee.com" {
        if REDINF_PATH_PREFIXES
            .iter()
            .any(|prefix| path.to_lowercase().starts_with(prefix))
        {
            "redinf".to_string()
        } else {
            "gitee".to_string()
        }
    } else if host == "192.168.0.101" {
        "avic".to_string()
    } else {
        "origin".to_string()
    };
    Some(remote_name)
}
