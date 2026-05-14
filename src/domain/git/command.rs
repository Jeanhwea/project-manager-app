use super::{GitError, Result};
use crate::domain::runner::{CommandResult, CommandRunner, ExecutionContext, OutputMode};
use std::path::Path;
use std::process::{ExitStatus, Output as ProcessOutput};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt;

pub struct GitCommandRunner;

impl GitCommandRunner {
    pub fn new() -> Self {
        Self
    }

    fn run(&self, context: &ExecutionContext) -> Result<CommandResult> {
        CommandRunner
            .execute(context)
            .map_err(|e| GitError::CommandFailed(e.to_string()))
    }

    pub fn execute(&self, args: &[&str], dir: Option<&Path>) -> Result<String> {
        let mut ctx = ExecutionContext::new("git")
            .args(args.iter().copied())
            .output_mode(OutputMode::Capture);

        if let Some(dir) = dir {
            ctx = ctx.working_dir(dir);
        }

        let result = self.run(&ctx)?;

        if !result.success {
            let stderr = result.stderr.unwrap_or_default();
            return Err(GitError::CommandFailed(stderr));
        }

        Ok(result.stdout.unwrap_or_default().trim().to_string())
    }

    pub fn execute_with_success(&self, args: &[&str], dir: Option<&Path>) -> Result<()> {
        let mut ctx = ExecutionContext::new("git")
            .args(args.iter().copied())
            .output_mode(OutputMode::Capture);

        if let Some(dir) = dir {
            ctx = ctx.working_dir(dir);
        }

        let result = self.run(&ctx)?;

        if !result.success {
            let stderr = result.stderr.unwrap_or_default();
            return Err(GitError::CommandFailed(format!(
                "Git command failed: {}",
                stderr.trim()
            )));
        }
        Ok(())
    }

    pub fn execute_streaming(&self, args: &[&str], dir: &Path) -> Result<()> {
        let ctx = ExecutionContext::new("git")
            .args(args.iter().copied())
            .working_dir(dir)
            .output_mode(OutputMode::Streaming);

        let result = self.run(&ctx)?;

        if !result.success {
            return Err(GitError::CommandFailed(format!(
                "Git command exited with code {}",
                result.exit_code
            )));
        }

        Ok(())
    }

    pub fn execute_raw(&self, args: &[&str], dir: &Path) -> Result<ProcessOutput> {
        let ctx = ExecutionContext::new("git")
            .args(args.iter().copied())
            .working_dir(dir)
            .output_mode(OutputMode::Capture);

        let result = self.run(&ctx)?;

        #[cfg(unix)]
        let status = ExitStatus::from_raw(result.exit_code);
        #[cfg(windows)]
        let status = ExitStatus::from_raw(result.exit_code as u32);

        let stdout = result.stdout.unwrap_or_default().into_bytes();
        let stderr = result.stderr.unwrap_or_default().into_bytes();

        Ok(ProcessOutput {
            status,
            stdout,
            stderr,
        })
    }

    pub fn get_current_branch(&self, repo_path: &Path) -> Result<String> {
        self.execute(&["branch", "--show-current"], Some(repo_path))
    }

    pub fn get_remote_list(&self, repo_path: &Path) -> Result<Vec<String>> {
        let output = self.execute(&["remote"], Some(repo_path))?;
        Ok(output
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect())
    }

    pub fn has_uncommitted_changes(&self, repo_path: &Path) -> Result<bool> {
        let output = self.execute(&["status", "--porcelain"], Some(repo_path))?;
        Ok(!output.is_empty())
    }

    pub fn is_merged_branch(&self, name: &str, repo_path: &Path) -> bool {
        self.execute(&["branch", "--merged", "master"], Some(repo_path))
            .map(|output| {
                output
                    .lines()
                    .any(|line| line.trim_start_matches("* ").trim() == name)
            })
            .unwrap_or(false)
    }

    pub fn execute_operation(&self, op: &crate::model::plan::GitOperation, ctx: &crate::model::plan::GitOperationContext) -> Result<()> {
        match op {
            crate::model::plan::GitOperation::Init => {
                self.execute_with_success(&["init"], Some(&ctx.working_dir))
            }
            crate::model::plan::GitOperation::Clone { url, target_dir } => {
                self.execute_with_success(&["clone", url, target_dir.to_string_lossy().as_ref()], Some(&ctx.working_dir))
            }
            crate::model::plan::GitOperation::Add { path } => {
                self.execute_with_success(&["add", path], Some(&ctx.working_dir))
            }
            crate::model::plan::GitOperation::Commit { message } => {
                self.execute_with_success(&["commit", "-m", message], Some(&ctx.working_dir))
            }
            crate::model::plan::GitOperation::CreateTag { tag } => {
                self.execute_with_success(&["tag", tag], Some(&ctx.working_dir))
            }
            crate::model::plan::GitOperation::PushTag { remote, tag } => {
                self.execute_with_success(&["push", remote, tag], Some(&ctx.working_dir))
            }
            crate::model::plan::GitOperation::PushBranch { remote, branch } => {
                self.execute_with_success(&["push", remote, branch], Some(&ctx.working_dir))
            }
            crate::model::plan::GitOperation::PushAll { remote } => {
                self.execute_with_success(&["push", "--all", remote], Some(&ctx.working_dir))
            }
            crate::model::plan::GitOperation::PushTags { remote } => {
                self.execute_with_success(&["push", "--tags", remote], Some(&ctx.working_dir))
            }
            crate::model::plan::GitOperation::Pull { remote, branch } => {
                self.execute_with_success(&["pull", remote, branch], Some(&ctx.working_dir))
            }
            crate::model::plan::GitOperation::Checkout { ref_name } => {
                self.execute_with_success(&["checkout", ref_name], Some(&ctx.working_dir))
            }
            crate::model::plan::GitOperation::DeleteBranch { branch } => {
                self.execute_with_success(&["branch", "-d", branch], Some(&ctx.working_dir))
            }
            crate::model::plan::GitOperation::RenameBranch { old, new } => {
                self.execute_with_success(&["branch", "-m", old, new], Some(&ctx.working_dir))
            }
            crate::model::plan::GitOperation::DeleteRemoteBranch { remote, branch } => {
                self.execute_with_success(&["push", remote, "--delete", branch], Some(&ctx.working_dir))
            }
            crate::model::plan::GitOperation::RenameRemote { old, new } => {
                self.execute_with_success(&["remote", "rename", old, new], Some(&ctx.working_dir))
            }
            crate::model::plan::GitOperation::PruneRemote { remote } => {
                self.execute_with_success(&["remote", "prune", remote], Some(&ctx.working_dir))
            }
            crate::model::plan::GitOperation::SetUpstream { remote, branch } => {
                self.execute_with_success(&["branch", "--set-upstream-to", &format!("{}/{}", remote, branch)], Some(&ctx.working_dir))
            }
            crate::model::plan::GitOperation::Gc => {
                self.execute_with_success(&["gc", "--aggressive"], Some(&ctx.working_dir))
            }
        }
    }
}

impl Default for GitCommandRunner {
    fn default() -> Self {
        Self::new()
    }
}
