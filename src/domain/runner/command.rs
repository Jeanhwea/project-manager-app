use std::io::{BufRead, BufReader};
use std::process::{Command as StdCommand, Stdio};
use std::thread;

use super::{CommandResult, ExecutionContext, OutputMode};
use crate::error::{AppError, Result};

pub struct CommandRunner;

impl CommandRunner {
    pub fn execute(&self, context: &ExecutionContext) -> Result<CommandResult> {
        match context.output_mode {
            OutputMode::Capture => self.execute_capture(context),
            OutputMode::Streaming => self.execute_streaming(context),
        }
    }

    fn execute_capture(&self, context: &ExecutionContext) -> Result<CommandResult> {
        let mut cmd = self.build_command(context)?;

        let output = cmd.output().map_err(AppError::Io)?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

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

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AppError::invalid_input("Failed to capture stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| AppError::invalid_input("Failed to capture stderr"))?;

        let stdout_thread = thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(std::result::Result::ok) {
                println!("{}", line);
            }
        });

        let stderr_thread = thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(std::result::Result::ok) {
                eprintln!("{}", line);
            }
        });

        let status = child.wait().map_err(AppError::Io)?;

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
        let mut cmd = StdCommand::new(&context.program);
        cmd.args(&context.args);

        if let Some(dir) = &context.working_dir {
            cmd.current_dir(dir);
        }

        #[cfg(target_os = "windows")]
        {
            // On Windows, ensure we have the system PATH
            // First try to get PATH from environment
            if let Ok(path) = std::env::var("PATH") {
                cmd.env("PATH", path);
            } else {

                // If we can't get PATH, don't set it explicitly to avoid overriding
                // the default inheritance behavior
            }
        }

        Ok(cmd)
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
