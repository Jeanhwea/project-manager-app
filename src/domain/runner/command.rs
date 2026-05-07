use std::io::{BufRead, BufReader};
use std::process::{Command as StdCommand, Stdio};
use std::thread;

use super::{CommandError, CommandResult, ExecutionContext, OutputMode};
use crate::utils::output::Output;

pub trait CommandRunner: Send + Sync {
    fn execute(&self, context: &ExecutionContext) -> Result<CommandResult, CommandError>;

    fn execute_streaming(
        &self,
        context: &ExecutionContext,
    ) -> Result<CommandResult, CommandError> {
        let ctx = context.clone().output_mode(OutputMode::Streaming);
        self.execute(&ctx)
    }

    fn execute_capture(&self, context: &ExecutionContext) -> Result<CommandResult, CommandError> {
        let ctx = context.clone().output_mode(OutputMode::Capture);
        self.execute(&ctx)
    }

    fn execute_dry_run(&self, context: &ExecutionContext) -> Result<CommandResult, CommandError> {
        let ctx = context.clone().output_mode(OutputMode::DryRun);
        self.execute(&ctx)
    }
}

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

    fn execute_streaming_impl(
        &self,
        context: &ExecutionContext,
    ) -> Result<CommandResult, CommandError> {
        let mut cmd = self.build_command(context)?;

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| CommandError::FailedToStart {
            program: context.program.clone(),
            reason: e.to_string(),
        })?;

        let stdout = child.stdout.take().ok_or_else(|| CommandError::IoError {
            message: "Failed to capture stdout".to_string(),
        })?;
        let stderr = child.stderr.take().ok_or_else(|| CommandError::IoError {
            message: "Failed to capture stderr".to_string(),
        })?;

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

        let status = child.wait().map_err(|e| CommandError::ExecutionFailed {
            program: context.program.clone(),
            reason: e.to_string(),
        })?;

        let _ = stdout_thread.join();
        let _ = stderr_thread.join();

        Ok(CommandResult {
            exit_code: status.code().unwrap_or(-1),
            success: status.success(),
            stdout: None,
            stderr: None,
        })
    }

    fn execute_dry_run_impl(
        &self,
        context: &ExecutionContext,
    ) -> Result<CommandResult, CommandError> {
        let cmd_str = self.format_command(context);
        Output::cmd(&format!("[DRY-RUN] {}", cmd_str));

        Ok(CommandResult::success())
    }

    fn build_command(&self, context: &ExecutionContext) -> Result<StdCommand, CommandError> {
        let mut cmd = StdCommand::new(&context.program);
        cmd.args(&context.args);

        if let Some(dir) = &context.working_dir {
            cmd.current_dir(dir);
        }

        for (key, value) in &context.env_vars {
            cmd.env(key, value);
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
    use std::sync::RwLock;

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

        assert!(true);
    }

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

        assert!(result.is_err());
    }

    #[test]
    fn test_exit_code_preservation() {
        let runner = DefaultCommandRunner;
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
