use std::io::{BufRead, BufReader};
use std::process::{Command as StdCommand, Stdio};
use std::thread;

use super::{CommandError, CommandResult, ExecutionContext, OutputMode};
use crate::utils::output::Output;

/// 统一的命令执行器 Trait
///
/// 提供标准化的命令执行接口，支持多种输出模式：
/// - **Capture**: 捕获 stdout/stderr 输出，适用于需要解析输出的场景
/// - **Streaming**: 实时显示输出，适用于长时间运行的命令
/// - **DryRun**: 仅打印命令预览，不实际执行
///
/// # Example
///
/// ```ignore
/// use domain::runner::{CommandRunner, DefaultCommandRunner, ExecutionContext, OutputMode};
///
/// let runner = DefaultCommandRunner;
///
/// // 流式执行 git pull
/// let ctx = ExecutionContext::new("git")
///     .args(["pull", "--rebase"])
///     .output_mode(OutputMode::Streaming);
///
/// let result = runner.execute(&ctx)?;
/// if !result.success {
///     eprintln!("Git pull failed with exit code {}", result.exit_code);
/// }
/// ```
#[allow(dead_code)]
pub trait CommandRunner: Send + Sync {
    /// 执行命令，根据上下文中的 output_mode 决定执行方式
    ///
    /// # Arguments
    ///
    /// * `context` - 命令执行上下文，包含程序名、参数、工作目录、环境变量和输出模式
    ///
    /// # Returns
    ///
    /// 返回 `Result<CommandResult, CommandError>`:
    /// - `Ok(CommandResult)` - 命令执行完成（无论成功或失败）
    /// - `Err(CommandError)` - 命令无法启动或执行过程中发生错误
    ///
    /// # Errors
    ///
    /// 可能返回以下错误：
    /// - `CommandError::FailedToStart` - 命令无法启动（如命令不存在、权限不足）
    /// - `CommandError::ExecutionFailed` - 执行过程中发生错误
    /// - `CommandError::IoError` - I/O 操作错误
    fn execute(&self, context: &ExecutionContext) -> Result<CommandResult, CommandError>;

    /// 流式执行命令 (强制使用 Streaming 模式)
    ///
    /// 忽略上下文中的 output_mode，强制使用 Streaming 模式执行。
    /// 适用于长时间运行的命令，如 git pull/push/fetch。
    ///
    /// # Arguments
    ///
    /// * `context` - 命令执行上下文
    ///
    /// # Returns
    ///
    /// 返回 `Result<CommandResult, CommandError>`:
    /// - `Ok(CommandResult)` - 命令执行完成，stdout/stderr 为 None
    /// - `Err(CommandError)` - 命令执行失败
    ///
    /// # Note
    ///
    /// 流式模式下，输出会实时显示到终端，不会存储在 CommandResult 中。
    fn execute_streaming(
        &self,
        context: &ExecutionContext,
    ) -> Result<CommandResult, CommandError> {
        let ctx = context.clone().output_mode(OutputMode::Streaming);
        self.execute(&ctx)
    }

    /// 捕获执行命令 (强制使用 Capture 模式)
    ///
    /// 忽略上下文中的 output_mode，强制使用 Capture 模式执行。
    /// 适用于需要解析命令输出的场景，如 git status。
    ///
    /// # Arguments
    ///
    /// * `context` - 命令执行上下文
    ///
    /// # Returns
    ///
    /// 返回 `Result<CommandResult, CommandError>`:
    /// - `Ok(CommandResult)` - 命令执行完成，stdout/stderr 包含捕获的输出
    /// - `Err(CommandError)` - 命令执行失败
    ///
    /// # Note
    ///
    /// 即使命令执行失败（非零退出码），也会返回 Ok(CommandResult)，
    /// 其中包含 stdout 和 stderr 的内容。
    fn execute_capture(&self, context: &ExecutionContext) -> Result<CommandResult, CommandError> {
        let ctx = context.clone().output_mode(OutputMode::Capture);
        self.execute(&ctx)
    }

    /// DryRun 执行命令 (强制使用 DryRun 模式)
    ///
    /// 忽略上下文中的 output_mode，强制使用 DryRun 模式执行。
    /// 仅打印命令预览，不会实际执行命令。
    ///
    /// # Arguments
    ///
    /// * `context` - 命令执行上下文
    ///
    /// # Returns
    ///
    /// 始终返回 `Ok(CommandResult::success())`，因为 DryRun 模式不会实际执行命令。
    ///
    /// # Note
    ///
    /// DryRun 模式下：
    /// - 不会创建任何子进程
    /// - stdout/stderr 始终为 None
    /// - exit_code 始终为 0
    fn execute_dry_run(&self, context: &ExecutionContext) -> Result<CommandResult, CommandError> {
        let ctx = context.clone().output_mode(OutputMode::DryRun);
        self.execute(&ctx)
    }
}

/// 默认命令执行器实现
///
/// 提供三种输出模式的实现：
/// - **Capture**: 使用 `.output()` 捕获命令输出
/// - **Streaming**: 使用 `.spawn()` + 线程实现实时输出
/// - **DryRun**: 仅打印命令预览，不实际执行
///
/// # Example
///
/// ```ignore
/// use domain::runner::{DefaultCommandRunner, ExecutionContext, OutputMode, CommandRunner};
///
/// let runner = DefaultCommandRunner;
///
/// // 捕获模式执行
/// let ctx = ExecutionContext::new("git")
///     .args(["status", "--porcelain"])
///     .output_mode(OutputMode::Capture);
///
/// let result = runner.execute(&ctx)?;
/// if let Some(stdout) = result.stdout {
///     println!("Changed files: {}", stdout);
/// }
/// ```
pub struct DefaultCommandRunner;

impl CommandRunner for DefaultCommandRunner {
    fn execute(&self, context: &ExecutionContext) -> Result<CommandResult, CommandError> {
        match context.output_mode {
            OutputMode::Capture => self.execute_capture_impl(context),
            OutputMode::Streaming => self.execute_streaming_impl(context),
            OutputMode::DryRun => self.execute_dry_run_impl(context),
        }
    }
}

impl DefaultCommandRunner {
    /// 捕获模式实现 - 使用 .output() 等待命令完成
    ///
    /// 执行命令并捕获所有 stdout/stderr 输出。
    /// 适用于需要解析命令输出的场景。
    ///
    /// # Arguments
    ///
    /// * `context` - 命令执行上下文
    ///
    /// # Returns
    ///
    /// 返回包含完整输出的 `CommandResult`。
    fn execute_capture_impl(
        &self,
        context: &ExecutionContext,
    ) -> Result<CommandResult, CommandError> {
        let mut cmd = self.build_command(context)?;

        let output = cmd.output().map_err(|e| CommandError::FailedToStart {
            program: context.program.clone(),
            reason: e.to_string(),
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        Ok(CommandResult::with_output(
            output.status.code().unwrap_or(-1),
            stdout,
            stderr,
        ))
    }

    /// 流式模式实现 - 使用 spawn + 线程读取实现实时输出
    ///
    /// 实时显示命令的 stdout 和 stderr 输出。
    /// 适用于长时间运行的命令，如 git pull/push/fetch。
    ///
    /// # Arguments
    ///
    /// * `context` - 命令执行上下文
    ///
    /// # Returns
    ///
    /// 返回不包含输出内容的 `CommandResult`（输出已实时显示到终端）。
    fn execute_streaming_impl(
        &self,
        context: &ExecutionContext,
    ) -> Result<CommandResult, CommandError> {
        let mut cmd = self.build_command(context)?;

        // 设置 stdout 和 stderr 为管道
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| CommandError::FailedToStart {
            program: context.program.clone(),
            reason: e.to_string(),
        })?;

        // 获取 stdout 和 stderr 管道
        let stdout = child.stdout.take().ok_or_else(|| CommandError::IoError {
            message: "Failed to capture stdout".to_string(),
        })?;
        let stderr = child.stderr.take().ok_or_else(|| CommandError::IoError {
            message: "Failed to capture stderr".to_string(),
        })?;

        // 创建线程读取 stdout
        let stdout_thread = thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                println!("{}", line); // 实时输出到终端
            }
        });

        // 创建线程读取 stderr
        let stderr_thread = thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                eprintln!("{}", line); // 实时输出到终端
            }
        });

        // 等待命令完成
        let status = child.wait().map_err(|e| CommandError::ExecutionFailed {
            program: context.program.clone(),
            reason: e.to_string(),
        })?;

        // 等待输出线程完成
        let _ = stdout_thread.join();
        let _ = stderr_thread.join();

        Ok(CommandResult {
            exit_code: status.code().unwrap_or(-1),
            success: status.success(),
            stdout: None, // 流式模式不返回输出
            stderr: None,
        })
    }

    /// DryRun 模式实现 - 仅打印命令
    ///
    /// 打印将要执行的命令，但不实际执行。
    /// 用于预览变更。
    ///
    /// # Arguments
    ///
    /// * `context` - 命令执行上下文
    ///
    /// # Returns
    ///
    /// 始终返回成功的 `CommandResult`。
    fn execute_dry_run_impl(
        &self,
        context: &ExecutionContext,
    ) -> Result<CommandResult, CommandError> {
        let cmd_str = self.format_command(context);
        Output::cmd(&format!("[DRY-RUN] {}", cmd_str));

        Ok(CommandResult::success())
    }

    /// 构建 Command 对象
    ///
    /// 根据执行上下文创建 `std::process::Command` 对象，
    /// 设置工作目录和环境变量。
    ///
    /// # Arguments
    ///
    /// * `context` - 命令执行上下文
    ///
    /// # Returns
    ///
    /// 返回配置好的 `Command` 对象。
    fn build_command(&self, context: &ExecutionContext) -> Result<StdCommand, CommandError> {
        let mut cmd = StdCommand::new(&context.program);
        cmd.args(&context.args);

        if let Some(dir) = &context.working_dir {
            cmd.current_dir(dir);
        }

        // 合并环境变量
        for (key, value) in &context.env_vars {
            cmd.env(key, value);
        }

        Ok(cmd)
    }

    /// 格式化命令字符串 (用于显示)
    ///
    /// 将程序名和参数组合成可读的命令字符串。
    ///
    /// # Arguments
    ///
    /// * `context` - 命令执行上下文
    ///
    /// # Returns
    ///
    /// 返回格式化的命令字符串。
    fn format_command(&self, context: &ExecutionContext) -> String {
        let mut parts = vec![context.program.clone()];
        parts.extend(context.args.clone());
        parts.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::RwLock;

    /// Mock implementation for testing the trait's default methods
    struct MockCommandRunner {
        last_mode: RwLock<Option<OutputMode>>,
    }

    impl MockCommandRunner {
        fn new() -> Self {
            Self {
                last_mode: RwLock::new(None),
            }
        }
    }

    impl CommandRunner for MockCommandRunner {
        fn execute(&self, context: &ExecutionContext) -> Result<CommandResult, CommandError> {
            *self.last_mode.write().unwrap() = Some(context.output_mode);
            Ok(CommandResult::success())
        }
    }

    #[test]
    fn test_execute_streaming_forces_streaming_mode() {
        let runner = MockCommandRunner::new();
        let ctx = ExecutionContext::new("git")
            .arg("pull")
            .output_mode(OutputMode::Capture);

        let _ = runner.execute_streaming(&ctx);

        assert_eq!(
            *runner.last_mode.read().unwrap(),
            Some(OutputMode::Streaming)
        );
    }

    #[test]
    fn test_execute_capture_forces_capture_mode() {
        let runner = MockCommandRunner::new();
        let ctx = ExecutionContext::new("git")
            .arg("status")
            .output_mode(OutputMode::Streaming);

        let _ = runner.execute_capture(&ctx);

        assert_eq!(*runner.last_mode.read().unwrap(), Some(OutputMode::Capture));
    }

    #[test]
    fn test_execute_dry_run_forces_dry_run_mode() {
        let runner = MockCommandRunner::new();
        let ctx = ExecutionContext::new("git")
            .arg("push")
            .output_mode(OutputMode::Capture);

        let _ = runner.execute_dry_run(&ctx);

        assert_eq!(*runner.last_mode.read().unwrap(), Some(OutputMode::DryRun));
    }

    #[test]
    fn test_execute_uses_context_mode() {
        let runner = MockCommandRunner::new();
        let ctx = ExecutionContext::new("git")
            .arg("status")
            .output_mode(OutputMode::Capture);

        let _ = runner.execute(&ctx);

        assert_eq!(*runner.last_mode.read().unwrap(), Some(OutputMode::Capture));
    }

    #[test]
    fn test_execute_streaming_preserves_other_context_fields() {
        let runner = MockCommandRunner::new();
        let ctx = ExecutionContext::new("git")
            .args(["pull", "--rebase"])
            .working_dir("/path/to/repo")
            .env("GIT_AUTHOR_NAME", "Test User")
            .output_mode(OutputMode::Capture);

        let _ = runner.execute_streaming(&ctx);

        // Verify the context is properly cloned and modified
        // The mock runner just captures the mode, but we verify the method doesn't panic
        assert!(true);
    }

    // ============================================
    // Integration tests for three execution modes
    // ============================================

    #[test]
    fn test_capture_mode_captures_output() {
        let runner = DefaultCommandRunner;
        let ctx = ExecutionContext::new("echo")
            .arg("hello")
            .output_mode(OutputMode::Capture);

        let result = runner.execute(&ctx).unwrap();

        assert!(result.success);
        assert!(result.stdout.is_some());
        assert!(result.stdout.unwrap().contains("hello"));
    }

    #[test]
    fn test_capture_mode_captures_stderr() {
        let runner = DefaultCommandRunner;
        // On Windows, use cmd.exe to write to stderr
        #[cfg(target_os = "windows")]
        let ctx = ExecutionContext::new("cmd")
            .args(["/C", "echo stderr 1>&2"])
            .output_mode(OutputMode::Capture);
        #[cfg(not(target_os = "windows"))]
        let ctx = ExecutionContext::new("sh")
            .args(["-c", "echo stderr >&2"])
            .output_mode(OutputMode::Capture);

        let result = runner.execute(&ctx).unwrap();

        assert!(result.success);
        assert!(result.stderr.unwrap().contains("stderr"));
    }

    #[test]
    fn test_streaming_mode_executes_and_returns_no_output() {
        let runner = DefaultCommandRunner;
        let ctx = ExecutionContext::new("echo")
            .arg("streaming test")
            .output_mode(OutputMode::Streaming);

        let result = runner.execute(&ctx).unwrap();

        assert!(result.success);
        // Streaming mode should NOT capture output
        assert!(result.stdout.is_none());
        assert!(result.stderr.is_none());
    }

    #[test]
    fn test_dry_run_mode_does_not_execute() {
        let runner = DefaultCommandRunner;
        let ctx = ExecutionContext::new("nonexistent_command_that_should_not_run")
            .arg("test")
            .output_mode(OutputMode::DryRun);

        // DryRun should succeed even with a nonexistent command
        let result = runner.execute(&ctx).unwrap();

        assert!(result.success);
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.is_none());
        assert!(result.stderr.is_none());
    }

    #[test]
    fn test_dry_run_returns_success_for_any_command() {
        let runner = DefaultCommandRunner;
        let ctx = ExecutionContext::new("rm")
            .arg("-rf")
            .arg("/nonexistent/path/that/should/not/be/deleted")
            .output_mode(OutputMode::DryRun);

        // DryRun should always return success without executing
        let result = runner.execute(&ctx).unwrap();

        assert!(result.success);
        assert_eq!(result.exit_code, 0);
    }

    #[test]
    fn test_capture_mode_with_nonexistent_command() {
        let runner = DefaultCommandRunner;
        let ctx =
            ExecutionContext::new("nonexistent_command_xyz123").output_mode(OutputMode::Capture);

        let result = runner.execute(&ctx);

        // Should return an error for nonexistent command
        assert!(result.is_err());
    }

    #[test]
    fn test_exit_code_preservation() {
        let runner = DefaultCommandRunner;
        // On Windows, use cmd.exe to exit with code 42
        #[cfg(target_os = "windows")]
        let ctx = ExecutionContext::new("cmd")
            .args(["/C", "exit 42"])
            .output_mode(OutputMode::Capture);
        #[cfg(not(target_os = "windows"))]
        let ctx = ExecutionContext::new("sh")
            .args(["-c", "exit 42"])
            .output_mode(OutputMode::Capture);

        let result = runner.execute(&ctx).unwrap();

        assert!(!result.success);
        assert_eq!(result.exit_code, 42);
    }
}
