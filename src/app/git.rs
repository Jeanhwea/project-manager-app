use super::runner::CommandRunner;
use super::version::compare_versions;
use std::path::Path;

pub fn get_rev_revision(ref_name: &str) -> Option<String> {
    let output = CommandRunner::run_quiet("git", &["rev-parse", ref_name]).ok()?;

    let revision = String::from_utf8(output.stdout).ok()?;
    let revision = revision.trim();

    if revision.is_empty() {
        return None;
    }

    Some(revision.to_string())
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

pub fn add_file(file: &str) -> Result<(), String> {
    CommandRunner::run_with_success("git", &["add", file])?;
    Ok(())
}

pub fn commit(message: &str) -> Result<(), String> {
    CommandRunner::run_with_success("git", &["commit", "-m", message])?;
    Ok(())
}

pub fn create_tag(tag: &str) -> Result<(), String> {
    CommandRunner::run_with_success("git", &["tag", tag])?;
    Ok(())
}

pub fn push_tag(remote: &str, tag: &str) -> Result<(), String> {
    CommandRunner::run_with_success("git", &["push", remote, tag])?;
    Ok(())
}

pub fn push_branch(remote: &str, branch: &str) -> Result<(), String> {
    CommandRunner::run_with_success("git", &["push", remote, branch])?;
    Ok(())
}

pub fn get_remote_name(work_dir: &Path) -> Vec<String> {
    let output =
        match CommandRunner::run_quiet_in_dir("git", &["remote"], work_dir.to_str().unwrap()) {
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
            work_dir.to_str().unwrap(),
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
    } else if url.starts_with("https://") {
        "https".to_string()
    } else if url.starts_with("http://") {
        "http".to_string()
    } else {
        return None;
    };

    let url = if url.starts_with("git@") {
        url.replace("git@", "")
    } else if url.starts_with("https://") {
        url.replace("https://", "")
    } else if url.starts_with("http://") {
        url.replace("http://", "")
    } else {
        url
    };

    let parts: Vec<&str> = url.splitn(2, ':').collect();
    if parts.len() != 2 {
        return None;
    }

    let (remote, path) = (parts[0].to_string(), parts[1].to_string());
    Some((protocol, remote, path))
}
