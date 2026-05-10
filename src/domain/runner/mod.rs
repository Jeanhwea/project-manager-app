mod command;

pub use command::CommandRunner;

use std::path::{Path, PathBuf};

use crate::utils::output::Output;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputMode {
    #[default]
    Capture,
    Streaming,
    DryRun,
}

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub program: String,
    pub args: Vec<String>,
    pub working_dir: Option<PathBuf>,
    pub output_mode: OutputMode,
}

impl ExecutionContext {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            working_dir: None,
            output_mode: OutputMode::default(),
        }
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    pub fn output_mode(mut self, mode: OutputMode) -> Self {
        self.output_mode = mode;
        self
    }
}

#[derive(Debug)]
pub struct CommandResult {
    pub exit_code: i32,
    pub success: bool,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}

impl CommandResult {
    pub fn success() -> Self {
        Self {
            exit_code: 0,
            success: true,
            stdout: None,
            stderr: None,
        }
    }

    pub fn with_output(exit_code: i32, stdout: String, stderr: String) -> Self {
        Self {
            exit_code,
            success: exit_code == 0,
            stdout: Some(stdout),
            stderr: Some(stderr),
        }
    }
}

pub struct DryRunContext {
    dry_run: bool,
}

impl DryRunContext {
    pub fn new(dry_run: bool) -> Self {
        Self { dry_run }
    }

    pub fn is_dry_run(&self) -> bool {
        self.dry_run
    }

    pub fn run_in_dir(
        &self,
        program: &str,
        args: &[&str],
        dir: Option<&Path>,
    ) -> anyhow::Result<()> {
        let mode = if self.dry_run {
            OutputMode::DryRun
        } else {
            let cmd_str = format!("{} {}", program, args.join(" "));
            Output::cmd(&cmd_str);
            OutputMode::Streaming
        };

        let mut ctx = ExecutionContext::new(program)
            .args(args.iter().copied())
            .output_mode(mode);

        if let Some(dir) = dir {
            ctx = ctx.working_dir(dir);
        }

        let result = CommandRunner.execute(&ctx)?;

        if !result.success {
            anyhow::bail!("命令执行失败: {} {}", program, args.join(" "));
        }

        Ok(())
    }

    pub fn print_header(&self, msg: &str) {
        if self.dry_run {
            Output::dry_run_header(msg);
        }
    }

    pub fn print_message(&self, msg: &str) {
        if self.dry_run {
            Output::message(&format!("[DRY-RUN] {}", msg));
        }
    }
}
