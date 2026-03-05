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

pub fn get_remote_name(work_dir: &Path) -> Option<String> {
    let output =
        CommandRunner::run_quiet_in_dir("git", &["remote", "-v"], work_dir.to_str()?).ok()?;

    let remotes = String::from_utf8(output.stdout).ok()?;
    let remotes: Vec<&str> = remotes.lines().collect();

    if remotes.is_empty() {
        return None;
    }

    let first_remote = remotes.first()?;
    let remote_name = first_remote.split_whitespace().next()?;

    Some(remote_name.to_string())
}
