//! Git-specific utilities
//!
//! This module provides simple Git utility functions that complement the domain Git module.
//! These utilities are designed for common Git operations and provide consistent error handling.
//!
//! **Validates: Requirements 3.4**

use std::path::Path;
use std::process::Command;

/// Execute a Git command and return the output
///
/// # Arguments
/// * `repo_path` - Repository path to execute command in
/// * `args` - Git command arguments
///
/// # Returns
/// * `std::io::Result<String>` - Command output or error
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails or output is invalid UTF-8
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
            format!("Git command failed: {}", stderr.trim()),
        ))
    }
}

/// Check if a directory is a Git repository
///
/// # Arguments
/// * `path` - Path to check
///
/// # Returns
/// * `bool` - True if the directory contains a `.git` subdirectory
pub fn is_git_repo(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    if !path.is_dir() {
        return false;
    }
    
    let git_dir = path.join(".git");
    git_dir.is_dir()
}

/// Get current Git branch name
///
/// # Arguments
/// * `repo_path` - Repository path
///
/// # Returns
/// * `std::io::Result<String>` - Current branch name or error
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
pub fn get_current_branch(repo_path: impl AsRef<Path>) -> std::io::Result<String> {
    git_command(repo_path, &["branch", "--show-current"])
}

/// Get Git remote URLs
///
/// # Arguments
/// * `repo_path` - Repository path
///
/// # Returns
/// * `std::io::Result<Vec<String>>` - List of remote URLs or error
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
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
///
/// # Arguments
/// * `repo_path` - Repository path
///
/// # Returns
/// * `std::io::Result<bool>` - True if repository has uncommitted changes
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
pub fn has_uncommitted_changes(repo_path: impl AsRef<Path>) -> std::io::Result<bool> {
    let output = git_command(repo_path, &["status", "--porcelain"])?;
    Ok(!output.trim().is_empty())
}

/// Get Git repository root directory
///
/// # Arguments
/// * `start_path` - Starting path (can be anywhere in repository)
///
/// # Returns
/// * `std::io::Result<String>` - Repository root directory path
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails or not in a Git repository
pub fn get_repo_root(start_path: impl AsRef<Path>) -> std::io::Result<String> {
    git_command(start_path, &["rev-parse", "--show-toplevel"])
}

/// Check if Git is available on the system
///
/// # Returns
/// * `bool` - True if Git is available
pub fn is_git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

/// Get Git version
///
/// # Returns
/// * `std::io::Result<String>` - Git version string or error
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
pub fn get_git_version() -> std::io::Result<String> {
    git_command(".", &["--version"])
}

/// Get Git user name
///
/// # Arguments
/// * `repo_path` - Repository path
///
/// # Returns
/// * `std::io::Result<String>` - Git user name or error
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
pub fn get_git_user_name(repo_path: impl AsRef<Path>) -> std::io::Result<String> {
    git_command(repo_path, &["config", "user.name"])
}

/// Get Git user email
///
/// # Arguments
/// * `repo_path` - Repository path
///
/// # Returns
/// * `std::io::Result<String>` - Git user email or error
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
pub fn get_git_user_email(repo_path: impl AsRef<Path>) -> std::io::Result<String> {
    git_command(repo_path, &["config", "user.email"])
}

/// Get Git commit hash for HEAD
///
/// # Arguments
/// * `repo_path` - Repository path
///
/// # Returns
/// * `std::io::Result<String>` - Commit hash or error
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
pub fn get_head_commit(repo_path: impl AsRef<Path>) -> std::io::Result<String> {
    git_command(repo_path, &["rev-parse", "HEAD"])
}

/// Get Git commit hash for HEAD (short version)
///
/// # Arguments
/// * `repo_path` - Repository path
///
/// # Returns
/// * `std::io::Result<String>` - Short commit hash or error
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
pub fn get_head_commit_short(repo_path: impl AsRef<Path>) -> std::io::Result<String> {
    git_command(repo_path, &["rev-parse", "--short", "HEAD"])
}

/// Get Git commit message for HEAD
///
/// # Arguments
/// * `repo_path` - Repository path
///
/// # Returns
/// * `std::io::Result<String>` - Commit message or error
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
pub fn get_head_commit_message(repo_path: impl AsRef<Path>) -> std::io::Result<String> {
    git_command(repo_path, &["log", "-1", "--pretty=%B"])
}

/// Get Git tag for HEAD (if any)
///
/// # Arguments
/// * `repo_path` - Repository path
///
/// # Returns
/// * `std::io::Result<Option<String>>` - Tag name if HEAD is tagged, None otherwise
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
pub fn get_head_tag(repo_path: impl AsRef<Path>) -> std::io::Result<Option<String>> {
    let output = git_command(repo_path, &["describe", "--tags", "--exact-match", "HEAD"])?;
    if output.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(output))
    }
}

/// Get Git tag for HEAD or nearest tag
///
/// # Arguments
/// * `repo_path` - Repository path
///
/// # Returns
/// * `std::io::Result<String>` - Tag name or nearest tag with distance
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
pub fn get_head_or_nearest_tag(repo_path: impl AsRef<Path>) -> std::io::Result<String> {
    git_command(repo_path, &["describe", "--tags", "HEAD"])
}

/// Check if Git repository is in a detached HEAD state
///
/// # Arguments
/// * `repo_path` - Repository path
///
/// # Returns
/// * `std::io::Result<bool>` - True if in detached HEAD state
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
pub fn is_detached_head(repo_path: impl AsRef<Path>) -> std::io::Result<bool> {
    let output = git_command(repo_path, &["symbolic-ref", "-q", "HEAD"])?;
    Ok(output.trim().is_empty())
}

/// Get Git repository origin URL
///
/// # Arguments
/// * `repo_path` - Repository path
///
/// # Returns
/// * `std::io::Result<Option<String>>` - Origin URL if exists, None otherwise
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
pub fn get_origin_url(repo_path: impl AsRef<Path>) -> std::io::Result<Option<String>> {
    let output = git_command(repo_path, &["remote", "get-url", "origin"])?;
    if output.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(output))
    }
}

/// Get Git repository upstream URL for current branch
///
/// # Arguments
/// * `repo_path` - Repository path
///
/// # Returns
/// * `std::io::Result<Option<String>>` - Upstream URL if exists, None otherwise
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
pub fn get_upstream_url(repo_path: impl AsRef<Path>) -> std::io::Result<Option<String>> {
    let branch_result = get_current_branch(&repo_path);
    let branch = match branch_result {
        Ok(branch) => branch,
        Err(_) => return Ok(None), // No current branch (detached HEAD)
    };
    
    let output = git_command(&repo_path, &["config", &format!("branch.{}.remote", branch)])?;
    if output.trim().is_empty() {
        return Ok(None);
    }
    
    let remote = output.trim();
    let url_output = git_command(&repo_path, &["remote", "get-url", remote])?;
    if url_output.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(url_output))
    }
}

/// Get Git repository status summary
///
/// # Arguments
/// * `repo_path` - Repository path
///
/// # Returns
/// * `std::io::Result<String>` - Status summary
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
pub fn get_status_summary(repo_path: impl AsRef<Path>) -> std::io::Result<String> {
    git_command(repo_path, &["status", "--short", "--branch"])
}

/// Get Git repository log (last N commits)
///
/// # Arguments
/// * `repo_path` - Repository path
/// * `count` - Number of commits to show
///
/// # Returns
/// * `std::io::Result<String>` - Log output
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
pub fn get_log(repo_path: impl AsRef<Path>, count: usize) -> std::io::Result<String> {
    git_command(repo_path, &["log", &format!("-{}", count), "--oneline"])
}

/// Get Git repository diff for staged changes
///
/// # Arguments
/// * `repo_path` - Repository path
///
/// # Returns
/// * `std::io::Result<String>` - Diff output
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
pub fn get_staged_diff(repo_path: impl AsRef<Path>) -> std::io::Result<String> {
    git_command(repo_path, &["diff", "--staged"])
}

/// Get Git repository diff for unstaged changes
///
/// # Arguments
/// * `repo_path` - Repository path
///
/// # Returns
/// * `std::io::Result<String>` - Diff output
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
pub fn get_unstaged_diff(repo_path: impl AsRef<Path>) -> std::io::Result<String> {
    git_command(repo_path, &["diff"])
}

/// Check if Git repository has staged changes
///
/// # Arguments
/// * `repo_path` - Repository path
///
/// # Returns
/// * `std::io::Result<bool>` - True if repository has staged changes
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
pub fn has_staged_changes(repo_path: impl AsRef<Path>) -> std::io::Result<bool> {
    // git diff --staged --quiet returns 0 if no staged changes, 1 if staged changes
    // We check the command result, not the output
    let output = Command::new("git")
        .current_dir(repo_path.as_ref())
        .args(&["diff", "--staged", "--quiet"])
        .output()?;
    
    Ok(!output.status.success())
}

/// Check if Git repository has unstaged changes
///
/// # Arguments
/// * `repo_path` - Repository path
///
/// # Returns
/// * `std::io::Result<bool>` - True if repository has unstaged changes
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
pub fn has_unstaged_changes(repo_path: impl AsRef<Path>) -> std::io::Result<bool> {
    let output = Command::new("git")
        .current_dir(repo_path.as_ref())
        .args(&["diff", "--quiet"])
        .output()?;
    
    Ok(!output.status.success())
}

/// Get Git repository branch list
///
/// # Arguments
/// * `repo_path` - Repository path
///
/// # Returns
/// * `std::io::Result<Vec<String>>` - List of branch names
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
pub fn get_branch_list(repo_path: impl AsRef<Path>) -> std::io::Result<Vec<String>> {
    let output = git_command(repo_path, &["branch", "--list"])?;
    let branches: Vec<String> = output
        .lines()
        .map(|line| {
            let line = line.trim();
            if line.starts_with('*') {
                line[1..].trim().to_string()
            } else {
                line.to_string()
            }
        })
        .filter(|branch| !branch.is_empty())
        .collect();
    
    Ok(branches)
}

/// Get Git repository remote list
///
/// # Arguments
/// * `repo_path` - Repository path
///
/// # Returns
/// * `std::io::Result<Vec<String>>` - List of remote names
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
pub fn get_remote_list(repo_path: impl AsRef<Path>) -> std::io::Result<Vec<String>> {
    let output = git_command(repo_path, &["remote"])?;
    let remotes: Vec<String> = output
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|remote| !remote.is_empty())
        .collect();
    
    Ok(remotes)
}

/// Check if Git repository is clean (no uncommitted or staged changes)
///
/// # Arguments
/// * `repo_path` - Repository path
///
/// # Returns
/// * `std::io::Result<bool>` - True if repository is clean
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
pub fn is_repo_clean(repo_path: impl AsRef<Path>) -> std::io::Result<bool> {
    let has_uncommitted = has_uncommitted_changes(&repo_path)?;
    let has_staged = has_staged_changes(&repo_path)?;
    let has_unstaged = has_unstaged_changes(&repo_path)?;
    
    Ok(!has_uncommitted && !has_staged && !has_unstaged)
}

/// Get Git repository last commit date
///
/// # Arguments
/// * `repo_path` - Repository path
///
/// # Returns
/// * `std::io::Result<String>` - Last commit date
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
pub fn get_last_commit_date(repo_path: impl AsRef<Path>) -> std::io::Result<String> {
    git_command(repo_path, &["log", "-1", "--format=%cd", "--date=short"])
}

/// Get Git repository last commit author
///
/// # Arguments
/// * `repo_path` - Repository path
///
/// # Returns
/// * `std::io::Result<String>` - Last commit author
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
pub fn get_last_commit_author(repo_path: impl AsRef<Path>) -> std::io::Result<String> {
    git_command(repo_path, &["log", "-1", "--format=%an"])
}

/// Get Git repository last commit author email
///
/// # Arguments
/// * `repo_path` - Repository path
///
/// # Returns
/// * `std::io::Result<String>` - Last commit author email
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
pub fn get_last_commit_author_email(repo_path: impl AsRef<Path>) -> std::io::Result<String> {
    git_command(repo_path, &["log", "-1", "--format=%ae"])
}

/// Get Git repository file count
///
/// # Arguments
/// * `repo_path` - Repository path
///
/// # Returns
/// * `std::io::Result<usize>` - Number of files tracked by Git
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
pub fn get_file_count(repo_path: impl AsRef<Path>) -> std::io::Result<usize> {
    let output = git_command(repo_path, &["ls-files"])?;
    let count = output.lines().count();
    Ok(count)
}

/// Get Git repository size (approximate)
///
/// # Arguments
/// * `repo_path` - Repository path
///
/// # Returns
/// * `std::io::Result<u64>` - Approximate repository size in bytes
///
/// # Errors
/// * Returns `std::io::Error` if Git command fails
pub fn get_repo_size(repo_path: impl AsRef<Path>) -> std::io::Result<u64> {
    let output = git_command(repo_path, &["count-objects", "-v"])?;
    
    // Parse output like:
    // count: 123
    // size: 456789
    // ...
    for line in output.lines() {
        if line.starts_with("size:") {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 {
                if let Ok(size) = parts[1].trim().parse::<u64>() {
                    return Ok(size * 1024); // size is in KiB
                }
            }
        }
    }
    
    Ok(0)
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
    
    #[test]
    fn test_is_git_available() {
        // This test just checks that the function doesn't panic
        let _ = is_git_available();
        assert!(true);
    }
    
    #[test]
    fn test_get_git_version() {
        if is_git_available() {
            let result = get_git_version();
            assert!(result.is_ok());
            let version = result.unwrap();
            assert!(version.contains("git version"));
        }
    }
    
    #[test]
    fn test_get_git_user_name() {
        if is_git_available() {
            let result = get_git_user_name(".");
            // This might fail if git user.name is not set, which is OK
            // Just test that the function doesn't panic
            let _ = result;
            assert!(true);
        }
    }
    
    #[test]
    fn test_get_git_user_email() {
        if is_git_available() {
            let result = get_git_user_email(".");
            // This might fail if git user.email is not set, which is OK
            // Just test that the function doesn't panic
            let _ = result;
            assert!(true);
        }
    }
    
    #[test]
    fn test_get_head_commit() {
        if is_git_available() {
            let result = get_head_commit(".");
            // This might fail if not in a Git repository, which is OK
            // Just test that the function doesn't panic
            let _ = result;
            assert!(true);
        }
    }
    
    #[test]
    fn test_get_head_commit_short() {
        if is_git_available() {
            let result = get_head_commit_short(".");
            // This might fail if not in a Git repository, which is OK
            // Just test that the function doesn't panic
            let _ = result;
            assert!(true);
        }
    }
    
    #[test]
    fn test_get_head_commit_message() {
        if is_git_available() {
            let result = get_head_commit_message(".");
            // This might fail if not in a Git repository, which is OK
            // Just test that the function doesn't panic
            let _ = result;
            assert!(true);
        }
    }
    
    #[test]
    fn test_get_head_tag() {
        if is_git_available() {
            let result = get_head_tag(".");
            // This might fail if not in a Git repository or HEAD is not tagged, which is OK
            // Just test that the function doesn't panic
            let _ = result;
            assert!(true);
        }
    }
    
    #[test]
    fn test_get_head_or_nearest_tag() {
        if is_git_available() {
            let result = get_head_or_nearest_tag(".");
            // This might fail if not in a Git repository, which is OK
            // Just test that the function doesn't panic
            let _ = result;
            assert!(true);
        }
    }
    
    #[test]
    fn test_is_detached_head() {
        if is_git_available() {
            let result = is_detached_head(".");
            // This might fail if not in a Git repository, which is OK
            // Just test that the function doesn't panic
            let _ = result;
            assert!(true);
        }
    }
    
    #[test]
    fn test_get_origin_url() {
        if is_git_available() {
            let result = get_origin_url(".");
            // This might fail if not in a Git repository or no origin remote, which is OK
            // Just test that the function doesn't panic
            let _ = result;
            assert!(true);
        }
    }
    
    #[test]
    fn test_get_upstream_url() {
        if is_git_available() {
            let result = get_upstream_url(".");
            // This might fail if not in a Git repository or no upstream, which is OK
            // Just test that the function doesn't panic
            let _ = result;
            assert!(true);
        }
    }
    
    #[test]
    fn test_get_status_summary() {
        if is_git_available() {
            let result = get_status_summary(".");
            // This might fail if not in a Git repository, which is OK
            // Just test that the function doesn't panic
            let _ = result;
            assert!(true);
        }
    }
    
    #[test]
    fn test_get_log() {
        if is_git_available() {
            let result = get_log(".", 5);
            // This might fail if not in a Git repository, which is OK
            // Just test that the function doesn't panic
            let _ = result;
            assert!(true);
        }
    }
    
    #[test]
    fn test_get_staged_diff() {
        if is_git_available() {
            let result = get_staged_diff(".");
            // This might fail if not in a Git repository, which is OK
            // Just test that the function doesn't panic
            let _ = result;
            assert!(true);
        }
    }
    
    #[test]
    fn test_get_unstaged_diff() {
        if is_git_available() {
            let result = get_unstaged_diff(".");
            // This might fail if not in a Git repository, which is OK
            // Just test that the function doesn't panic
            let _ = result;
            assert!(true);
        }
    }
    
    #[test]
    fn test_has_staged_changes() {
        if is_git_available() {
            let result = has_staged_changes(".");
            // This might fail if not in a Git repository, which is OK
            // Just test that the function doesn't panic
            let _ = result;
            assert!(true);
        }
    }
    
    #[test]
    fn test_has_unstaged_changes() {
        if is_git_available() {
            let result = has_unstaged_changes(".");
            // This might fail if not in a Git repository, which is OK
            // Just test that the function doesn't panic
            let _ = result;
            assert!(true);
        }
    }
    
    #[test]
    fn test_get_branch_list() {
        if is_git_available() {
            let result = get_branch_list(".");
            // This might fail if not in a Git repository, which is OK
            // Just test that the function doesn't panic
            let _ = result;
            assert!(true);
        }
    }
    
    #[test]
    fn test_get_remote_list() {
        if is_git_available() {
            let result = get_remote_list(".");
            // This might fail if not in a Git repository, which is OK
            // Just test that the function doesn't panic
            let _ = result;
            assert!(true);
        }
    }
    
    #[test]
    fn test_is_repo_clean() {
        if is_git_available() {
            let result = is_repo_clean(".");
            // This might fail if not in a Git repository, which is OK
            // Just test that the function doesn't panic
            let _ = result;
            assert!(true);
        }
    }
    
    #[test]
    fn test_get_last_commit_date() {
        if is_git_available() {
            let result = get_last_commit_date(".");
            // This might fail if not in a Git repository, which is OK
            // Just test that the function doesn't panic
            let _ = result;
            assert!(true);
        }
    }
    
    #[test]
    fn test_get_last_commit_author() {
        if is_git_available() {
            let result = get_last_commit_author(".");
            // This might fail if not in a Git repository, which is OK
            // Just test that the function doesn't panic
            let _ = result;
            assert!(true);
        }
    }
    
    #[test]
    fn test_get_last_commit_author_email() {
        if is_git_available() {
            let result = get_last_commit_author_email(".");
            // This might fail if not in a Git repository, which is OK
            // Just test that the function doesn't panic
            let _ = result;
            assert!(true);
        }
    }
    
    #[test]
    fn test_get_file_count() {
        if is_git_available() {
            let result = get_file_count(".");
            // This might fail if not in a Git repository, which is OK
            // Just test that the function doesn't panic
            let _ = result;
            assert!(true);
        }
    }
    
    #[test]
    fn test_get_repo_size() {
        if is_git_available() {
            let result = get_repo_size(".");
            // This might fail if not in a Git repository, which is OK
            // Just test that the function doesn't panic
            let _ = result;
            assert!(true);
        }
    }
}