use std::io::{BufRead, BufReader};
use std::process::{Command as StdCommand, Stdio};
use std::thread;

use super::{CommandResult, ExecutionContext, OutputMode};
use crate::utils::output::Output;

pub struct CommandRunner;

impl CommandRunner {
    pub fn execute(&self, context: &ExecutionContext) -> anyhow::Result<CommandResult> {
        match context.output_mode {
            OutputMode::Capture => self.execute_capture(context),
            OutputMode::Streaming => self.execute_streaming(context),
            OutputMode::DryRun => self.execute_dry_run(context),
        }
    }

    fn execute_capture(&self, context: &ExecutionContext) -> anyhow::Result<CommandResult> {
        let mut cmd = self.build_command(context)?;

        let output = cmd.output().map_err(|e| {
            anyhow::anyhow!("Failed to start command '{}': {}", context.program, e)
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        Ok(CommandResult::with_output(
            output.status.code().unwrap_or(-1),
            stdout,
            stderr,
        ))
    }

    fn execute_streaming(&self, context: &ExecutionContext) -> anyhow::Result<CommandResult> {
        let mut cmd = self.build_command(context)?;

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            anyhow::anyhow!("Failed to start command '{}': {}", context.program, e)
        })?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture stderr"))?;

        let stdout_thread = thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                println!("{}", line);
            }
        });

        let stderr_thread = thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                eprintln!("{}", line);
            }
        });

        let status = child
            .wait()
            .map_err(|e| anyhow::anyhow!("Command '{}' failed: {}", context.program, e))?;

        let _ = stdout_thread.join();
        let _ = stderr_thread.join();

        Ok(CommandResult {
            exit_code: status.code().unwrap_or(-1),
            success: status.success(),
            stdout: None,
            stderr: None,
        })
    }

    fn execute_dry_run(&self, context: &ExecutionContext) -> anyhow::Result<CommandResult> {
        let cmd_str = self.format_command(context);
        Output::cmd(&format!("[DRY-RUN] {}", cmd_str));

        Ok(CommandResult::success())
    }

    fn build_command(&self, context: &ExecutionContext) -> anyhow::Result<StdCommand> {
        let mut cmd = StdCommand::new(&context.program);
        cmd.args(&context.args);

        if let Some(dir) = &context.working_dir {
            cmd.current_dir(dir);
        }

        Ok(cmd)
    }

    fn format_command(&self, context: &ExecutionContext) -> String {
        let mut parts = vec![context.program.clone()];
        parts.extend(context.args.clone());
        parts.join(" ")
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
    fn test_dry_run_returns_success_for_any_command() {
        let runner = CommandRunner;
        let ctx = ExecutionContext::new("rm")
            .args(["-rf", "/nonexistent/path/that/should/not/be/deleted"])
            .output_mode(OutputMode::DryRun);

        let result = runner.execute(&ctx).unwrap();

        assert!(result.success);
        assert_eq!(result.exit_code, 0);
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
