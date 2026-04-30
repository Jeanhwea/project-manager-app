//! Git-specific utilities

use std::path::Path;
use std::process::Command;

/// Execute a Git command and return the output
pub fn git_command(repo_path: impl AsRef<Path>, args: &[&str]) -> std::io::Result<String> {
    let output = Command::new("git")
        .current_dir(repo_path.as_ref())
        .args(args)
        .output()?;
    
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Git command failed: {}", stderr),
        ))
    }
}

/// Check if a directory is a Git repository
pub fn is_git_repo(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    if !path.is_dir() {
        return false;
    }
    
    let git_dir = path.join(".git");
    git_dir.is_dir()
}

/// Get current Git branch name
pub fn get_current_branch(repo_path: impl AsRef<Path>) -> std::io::Result<String> {
    git_command(repo_path, &["branch", "--show-current"])
}

/// Get Git remote URLs
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

/// Get Git repository root directory
pub fn get_repo_root(start_path: impl AsRef<Path>) -> std::io::Result<String> {
    git_command(start_path, &["rev-parse", "--show-toplevel"])
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_is_git_repo() {
        let temp_dir = tempdir().unwrap();
        
        // Non-Git directory should return false
        assert!(!is_git_repo(temp_dir.path()));
        
        // TODO: Add test with actual Git repository
        // This would require creating a test Git repo
    }
    
    #[test]
    fn test_git_command_error() {
        let temp_dir = tempdir().unwrap();
        
        // Invalid Git command should return error
        let result = git_command(temp_dir.path(), &["invalid-command"]);
        assert!(result.is_err());
    }
}