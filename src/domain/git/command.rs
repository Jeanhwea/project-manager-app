//! git CLI 在代码中的唯一出口：所有子命令最终都经这里落地，便于统一的错误归类。
//! `run_local` 面向短查询类命令（捕获 stdout）；`run_streaming` 面向 push/pull/clone 这类长任务。

use super::{GitError, Result};
use crate::domain::runner::{CommandRunner, ExecutionContext, OutputMode};
use crate::error::AppError;
use std::path::Path;
use std::time::Duration;

/// git 命令执行器；本身无状态，只负责把调用最终转交给 `domain::runner`
pub struct GitCommandRunner;

impl GitCommandRunner {
    pub fn new() -> Self {
        Self
    }

    /// 执行 git 命令并返回 trim 后的 stdout；`dir` 为 None 时在进程当前目录执行，
    /// 退出码非 0 时转成 [`GitError::CommandFailed`]
    pub fn run_local(&self, args: &[&str], dir: Option<&Path>) -> Result<String> {
        let mut ctx = ExecutionContext::new("git")
            .args(args.iter().copied())
            .output_mode(OutputMode::Capture);

        if let Some(d) = dir {
            ctx = ctx.working_dir(d);
        }

        let result = CommandRunner
            .execute(&ctx)
            .map_err(|e| GitError::CommandFailed(e.to_string()))?;

        // 报错信息优先取 stderr：git 把失败原因写在 stderr，stdout 往往为空
        if !result.success {
            let stderr = result.stderr.unwrap_or_default().trim().to_string();
            return Err(GitError::CommandFailed(stderr));
        }
        Ok(result.stdout.unwrap_or_default().trim().to_string())
    }

    /// 执行耗时较长的 git 命令：输出实时转发到终端，因此拿不到 stdout；
    /// 只有这个入口支持 `timeout`，超时单独映射成 [`GitError::Timeout`]
    pub fn run_streaming(
        &self,
        args: &[&str],
        dir: &Path,
        timeout: Option<Duration>,
    ) -> Result<()> {
        let ctx = ExecutionContext::new("git")
            .args(args.iter().copied())
            .working_dir(dir)
            .output_mode(OutputMode::Streaming)
            .timeout(timeout);

        let result = match CommandRunner.execute(&ctx) {
            Ok(result) => result,
            Err(AppError::Timeout { .. }) => {
                let secs = timeout.map(|t| t.as_secs()).unwrap_or_default();
                return Err(GitError::Timeout {
                    timeout: crate::utils::format_duration(secs),
                });
            }
            Err(e) => return Err(GitError::CommandFailed(e.to_string())),
        };

        if !result.success {
            return Err(GitError::CommandFailed(format!(
                "Git command exited with code {}",
                result.exit_code
            )));
        }
        Ok(())
    }

    /// 当前分支名；detached HEAD 时 git 输出为空字符串
    pub fn current_branch(&self, repo_path: &Path) -> Result<String> {
        self.run_local(&["branch", "--show-current"], Some(repo_path))
    }

    pub fn remote_names(&self, repo_path: &Path) -> Result<Vec<String>> {
        let output = self.run_local(&["remote"], Some(repo_path))?;
        Ok(output
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect())
    }

    /// 是否存在未提交改动（含未跟踪文件）
    pub fn has_uncommitted_changes(&self, repo_path: &Path) -> Result<bool> {
        // porcelain 输出稳定且不受终端宽度、颜色配置影响，空输出即代表工作区干净
        let output = self.run_local(&["status", "--porcelain"], Some(repo_path))?;
        Ok(!output.is_empty())
    }
}

impl Default for GitCommandRunner {
    fn default() -> Self {
        Self::new()
    }
}
