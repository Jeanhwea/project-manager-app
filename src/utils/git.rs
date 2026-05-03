//! Git-specific utilities

use std::path::Path;
use std::process::Command;

/// Execute a Git command and return the output as a trimmed string
pub fn git_command(repo_path: impl AsRef<Path>, args: &[&str]) -> std::io::Result<String> {
    let output = Command::new("git")
        .current_dir(repo_path.as_ref())
        .args(args)
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(std::io::Error::other(format!(
            "Git command failed: {}",
            stderr.trim()
        )))
    }
}

/// Check if a directory is a Git repository
pub fn is_git_repo(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    path.is_dir() && path.join(".git").is_dir()
}

/// Get current Git branch name
pub fn get_current_branch(repo_path: impl AsRef<Path>) -> std::io::Result<String> {
    git_command(repo_path, &["branch", "--show-current"])
}

/// Get Git remote URLs (fetch + push)
pub fn get_remote_urls(repo_path: impl AsRef<Path>) -> std::io::Result<Vec<String>> {
    let output = git_command(repo_path, &["remote", "-v"])?;
    let urls: Vec<String> = output
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                Some(parts[1].to_string())
            } else {
                None
            }
        })
        .collect();
    Ok(urls)
}

/// Check if Git repository has uncommitted changes
pub fn has_uncommitted_changes(repo_path: impl AsRef<Path>) -> std::io::Result<bool> {
    let output = git_command(repo_path, &["status", "--porcelain"])?;
    Ok(!output.trim().is_empty())
}

/// Get Git remote name list
pub fn get_remote_list(repo_path: impl AsRef<Path>) -> std::io::Result<Vec<String>> {
    let output = git_command(repo_path, &["remote"])?;
    let remotes: Vec<String> = output
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|remote| !remote.is_empty())
        .collect();
    Ok(remotes)
}
