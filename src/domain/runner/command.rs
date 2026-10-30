//! `ExecutionContext` 的落地实现：按 OutputMode 分派到捕获或流式执行，并提供超时强杀。

use std::io::{BufRead, BufReader};
use std::process::{Child, Command as StdCommand, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::{CommandResult, ExecutionContext, OutputMode};
use crate::error::{AppError, Result};

/// 轮询等待子进程的间隔：不能用阻塞等待，否则就没机会中途 kill
const POLL_INTERVAL: Duration = Duration::from_millis(200);

/// 无状态的命令执行器
pub struct CommandRunner;

impl CommandRunner {
    /// 按上下文的输出方式分派到对应的执行路径
    pub fn execute(&self, context: &ExecutionContext) -> Result<CommandResult> {
        match context.output_mode {
            OutputMode::Capture => self.execute_capture(context),
            OutputMode::Streaming => self.execute_streaming(context),
        }
    }

    /// 捕获模式：`timeout` 在这里不生效，命令始终阻塞到结束，
    /// 因此只应用于能快速返回的查询类命令
    fn execute_capture(&self, context: &ExecutionContext) -> Result<CommandResult> {
        let mut cmd = self.build_command(context)?;

        let output = cmd.output().map_err(AppError::Io)?;

        // 非 UTF-8 输出替换成占位符而非直接失败，避免编码问题拖垮整条命令
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        // 进程被信号杀死时取不到退出码，统一记为 -1
        Ok(CommandResult::with_output(
            output.status.code().unwrap_or(-1),
            stdout,
            stderr,
        ))
    }

    fn execute_streaming(&self, context: &ExecutionContext) -> Result<CommandResult> {
        let mut cmd = self.build_command(context)?;

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(AppError::Io)?;

        let stdout = child.stdout.take().ok_or_else(|| AppError::InvalidInput {
            reason: "Failed to capture stdout".into(),
        })?;
        let stderr = child.stderr.take().ok_or_else(|| AppError::InvalidInput {
            reason: "Failed to capture stderr".into(),
        })?;

        let sink = crate::utils::output::current_sink();
        let stdout_sink = sink.clone();
        let stderr_sink = sink.clone();

        // stdout/stderr 各起一个线程持续抽走，否则管道缓冲区写满会让子进程阻塞
        let stdout_thread = thread::spawn(move || {
            let reader = BufReader::new(stdout);
            // 读到无效 UTF-8 或管道关闭即停止转发
            for line in reader.lines().map_while(std::result::Result::ok) {
                crate::utils::output::emit_line(stdout_sink.as_ref(), &line);
            }
        });

        let stderr_thread = thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(std::result::Result::ok) {
                crate::utils::output::emit_line(stderr_sink.as_ref(), &line);
            }
        });

        let status = match context.timeout {
            Some(timeout) => wait_with_timeout(&mut child, timeout)?,
            None => child.wait().map_err(AppError::Io)?,
        };

        // 显式 join：保证退出前最后几行输出已经打完，不会和汇总页交错
        let _ = stdout_thread.join();
        let _ = stderr_thread.join();

        Ok(CommandResult {
            exit_code: status.code().unwrap_or(-1),
            success: status.success(),
            stdout: None,
            stderr: None,
        })
    }

    fn build_command(&self, context: &ExecutionContext) -> Result<StdCommand> {
        #[cfg(target_os = "windows")]
        {
            // Windows 下统一借 cmd /c 执行，避免 .bat/.cmd 等不同可执行类型的解析差异
            let mut cmd = StdCommand::new("cmd");
            cmd.arg("/c").arg(&context.program).args(&context.args);

            if let Some(dir) = &context.working_dir {
                cmd.current_dir(dir);
            }

            Ok(cmd)
        }

        #[cfg(not(target_os = "windows"))]
        {
            let mut cmd = StdCommand::new(&context.program);
            cmd.args(&context.args);

            if let Some(dir) = &context.working_dir {
                cmd.current_dir(dir);
            }

            Ok(cmd)
        }
    }
}

/// 轮询等待子进程，超过 `timeout` 就杀掉进程树并返回 Timeout 错误
fn wait_with_timeout(child: &mut Child, timeout: Duration) -> Result<std::process::ExitStatus> {
    let start = Instant::now();
    loop {
        match child.try_wait().map_err(AppError::Io)? {
            Some(status) => return Ok(status),
            None => {
                if start.elapsed() >= timeout {
                    kill_process_tree(child);
                    // 必须再 wait 一次回收子进程，否则会留下僵尸进程
                    let _ = child.wait();
                    let secs = timeout.as_secs();
                    // 把 91s 这种零碎秒数上取整到整分钟，报给用户更好读
                    let display_secs = if secs >= 60 && !secs.is_multiple_of(60) {
                        (secs / 60 + 1) * 60
                    } else {
                        secs
                    };
                    return Err(AppError::Timeout {
                        timeout: crate::utils::format_duration(display_secs),
                    });
                }
                thread::sleep(POLL_INTERVAL);
            }
        }
    }
}

/// 终止子进程及其后代。
/// 注意 Unix 分支只 kill 了直接子进程，git 再 fork 出去的 helper（如 ssh 传输层）可能残留。
fn kill_process_tree(child: &mut Child) {
    #[cfg(target_os = "windows")]
    {
        let pid = child.id();
        let _ = StdCommand::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .output();
        let _ = child.kill();
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = child.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_capture_mode_captures_output() {
        let runner = CommandRunner;
        let ctx = ExecutionContext::new("git")
            .args(["--version"])
            .output_mode(OutputMode::Capture);

        let result = runner.execute(&ctx).unwrap();

        assert!(result.success);
        assert!(result.stdout.is_some());
        assert!(result.stdout.unwrap().contains("git version"));
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_capture_mode_captures_stderr() {
        let runner = CommandRunner;
        let ctx = ExecutionContext::new("sh")
            .args(["-c", "echo stderr >&2"])
            .output_mode(OutputMode::Capture);

        let result = runner.execute(&ctx).unwrap();

        assert!(result.success);
        assert!(result.stderr.unwrap().contains("stderr"));
    }

    #[test]
    fn test_streaming_mode_executes_and_returns_no_output() {
        let runner = CommandRunner;
        let ctx = ExecutionContext::new("echo")
            .args(["streaming test"])
            .output_mode(OutputMode::Streaming);

        let result = runner.execute(&ctx).unwrap();

        assert!(result.success);
        assert!(result.stdout.is_none());
        assert!(result.stderr.is_none());
    }

    #[test]
    fn test_capture_mode_with_nonexistent_command() {
        let runner = CommandRunner;
        let ctx =
            ExecutionContext::new("nonexistent_command_xyz123").output_mode(OutputMode::Capture);

        let result = runner.execute(&ctx);

        assert!(result.is_err());
    }

    #[test]
    fn test_streaming_timeout_kills_long_command() {
        let runner = CommandRunner;
        let ctx = long_running_ctx().timeout(Some(Duration::from_millis(300)));

        let start = Instant::now();
        let result = runner.execute(&ctx);

        assert!(matches!(result, Err(AppError::Timeout { .. })));
        assert!(start.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn test_streaming_completes_within_timeout() {
        let runner = CommandRunner;
        let ctx = ExecutionContext::new("echo")
            .args(["quick"])
            .output_mode(OutputMode::Streaming)
            .timeout(Some(Duration::from_secs(30)));

        let result = runner.execute(&ctx);

        assert!(result.is_ok());
    }

    #[cfg(target_os = "windows")]
    fn long_running_ctx() -> ExecutionContext {
        ExecutionContext::new("cmd")
            .args(["/c", "ping -n 30 127.0.0.1"])
            .output_mode(OutputMode::Streaming)
    }

    #[cfg(not(target_os = "windows"))]
    fn long_running_ctx() -> ExecutionContext {
        ExecutionContext::new("sh")
            .args(["-c", "sleep 30"])
            .output_mode(OutputMode::Streaming)
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_exit_code_preservation() {
        let runner = CommandRunner;
        let ctx = ExecutionContext::new("sh")
            .args(["-c", "exit 42"])
            .output_mode(OutputMode::Capture);

        let result = runner.execute(&ctx).unwrap();

        assert!(!result.success);
        assert_eq!(result.exit_code, 42);
    }
}
