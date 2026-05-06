use super::{GitError, Result};
use crate::utils::output::Output;
use std::path::Path;
use std::process::{Command, Output as ProcessOutput};

#[derive(Debug, Clone)]
pub struct GitCommandRunner;

impl GitCommandRunner {
    pub fn new() -> Self {
        Self
    }

    pub fn execute(&self, args: &[&str]) -> Result<String> {
        let output = self.execute_raw(args)?;
        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| GitError::CommandFailed(format!("Invalid UTF-8 in output: {}", e)))?;
        Ok(stdout.trim().to_string())
    }

    pub fn execute_in_dir(&self, args: &[&str], dir: &Path) -> Result<String> {
        let output = self.execute_raw_in_dir(args, dir)?;
        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| GitError::CommandFailed(format!("Invalid UTF-8 in output: {}", e)))?;
        Ok(stdout.trim().to_string())
    }

    pub fn execute_raw(&self, args: &[&str]) -> Result<ProcessOutput> {
        self.run(args, None, false)
    }

    pub fn execute_raw_in_dir(&self, args: &[&str], dir: &Path) -> Result<ProcessOutput> {
        self.run(args, Some(dir), false)
    }

    pub fn execute_with_success(&self, args: &[&str]) -> Result<()> {
        self.check_success(args, None)
    }

    pub fn execute_with_success_in_dir(&self, args: &[&str], dir: &Path) -> Result<()> {
        self.check_success(args, Some(dir))
    }

    pub fn execute_quiet_in_dir(&self, args: &[&str], dir: &Path) -> Result<ProcessOutput> {
        self.run(args, Some(dir), false)
    }

    fn run(&self, args: &[&str], dir: Option<&Path>, print_cmd: bool) -> Result<ProcessOutput> {
        if print_cmd {
            let cmd_str = format!("git {}", args.join(" "));
            Output::cmd(&cmd_str);
        }
        let mut cmd = Command::new("git");
        cmd.args(args);
        if let Some(dir) = dir {
            cmd.current_dir(dir);
        }
        cmd.output()
            .map_err(|e| GitError::CommandFailed(format!("Failed to execute git: {}", e)))
    }

    fn check_success(&self, args: &[&str], dir: Option<&Path>) -> Result<()> {
        let output = self.run(args, dir, true)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GitError::CommandFailed(format!(
                "Git command failed: {}",
                stderr.trim()
            )));
        }
        Ok(())
    }

    pub fn get_current_branch(&self, repo_path: &Path) -> Result<String> {
        self.execute_in_dir(&["branch", "--show-current"], repo_path)
    }

    pub fn get_remote_urls(&self, repo_path: &Path) -> Result<Vec<String>> {
        let output = self.execute_in_dir(&["remote", "-v"], repo_path)?;
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

    pub fn get_remote_list(&self, repo_path: &Path) -> Result<Vec<String>> {
        let output = self.execute_in_dir(&["remote"], repo_path)?;
        let remotes: Vec<String> = output
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|remote| !remote.is_empty())
            .collect();
        Ok(remotes)
    }

    pub fn has_uncommitted_changes(&self, repo_path: &Path) -> Result<bool> {
        let output = self.execute_in_dir(&["status", "--porcelain"], repo_path)?;
        Ok(!output.is_empty())
    }
}

impl Default for GitCommandRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_execute_in_nonexistent_dir() {
        let runner = GitCommandRunner::new();
        let dir = tempdir().unwrap().path().join("nonexistent");
        assert!(runner.execute_in_dir(&["status"], &dir).is_err());
    }

    #[test]
    fn test_execute_with_invalid_subcommand() {
        let runner = GitCommandRunner::new();
        assert!(runner.execute_with_success(&["no-such-command"]).is_err());
    }
}
