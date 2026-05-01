use super::{GitError, Result};
use std::path::Path;
use std::process::{Command, Output};

/// 封装 git 命令执行，统一错误处理。
#[derive(Debug, Clone)]
pub struct GitCommandRunner;

impl GitCommandRunner {
    pub fn new() -> Self {
        Self
    }

    /// 执行 git 命令，返回 stdout 字符串。
    pub fn execute(&self, args: &[&str]) -> Result<String> {
        let output = self.execute_raw(args)?;
        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| GitError::CommandFailed(format!("Invalid UTF-8 in output: {}", e)))?;
        Ok(stdout.trim().to_string())
    }

    /// 在指定目录执行 git 命令，返回 stdout 字符串。
    pub fn execute_in_dir(&self, args: &[&str], dir: &Path) -> Result<String> {
        let output = self.execute_raw_in_dir(args, dir)?;
        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| GitError::CommandFailed(format!("Invalid UTF-8 in output: {}", e)))?;
        Ok(stdout.trim().to_string())
    }

    /// 执行 git 命令，返回原始 Output。
    pub fn execute_raw(&self, args: &[&str]) -> Result<Output> {
        self.run(args, None)
    }

    /// 在指定目录执行 git 命令，返回原始 Output。
    pub fn execute_raw_in_dir(&self, args: &[&str], dir: &Path) -> Result<Output> {
        self.run(args, Some(dir))
    }

    /// 执行 git 命令并检查退出码。
    pub fn execute_with_success(&self, args: &[&str]) -> Result<()> {
        self.check_success(args, None)
    }

    /// 在指定目录执行 git 命令并检查退出码。
    pub fn execute_with_success_in_dir(&self, args: &[&str], dir: &Path) -> Result<()> {
        self.check_success(args, Some(dir))
    }

    /// 静默执行 git 命令（不检查退出码）。
    pub fn execute_quiet(&self, args: &[&str]) -> Result<Output> {
        self.run(args, None)
    }

    /// 在指定目录静默执行 git 命令。
    pub fn execute_quiet_in_dir(&self, args: &[&str], dir: &Path) -> Result<Output> {
        self.run(args, Some(dir))
    }

    pub fn is_git_available(&self) -> bool {
        Command::new("git")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok()
    }

    pub fn get_git_version(&self) -> Result<String> {
        self.execute(&["--version"])
    }

    // ── internal ───────────────────────────────────────────────────

    fn run(&self, args: &[&str], dir: Option<&Path>) -> Result<Output> {
        let mut cmd = Command::new("git");
        cmd.args(args);
        if let Some(dir) = dir {
            cmd.current_dir(dir);
        }
        cmd.output()
            .map_err(|e| GitError::CommandFailed(format!("Failed to execute git: {}", e)))
    }

    fn check_success(&self, args: &[&str], dir: Option<&Path>) -> Result<()> {
        let output = self.run(args, dir)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GitError::CommandFailed(format!(
                "Git command failed: {}",
                stderr.trim()
            )));
        }
        Ok(())
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
    fn test_git_version() {
        let runner = GitCommandRunner::new();
        if runner.is_git_available() {
            let version = runner.get_git_version().unwrap();
            assert!(version.contains("git version"));
        }
    }

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
