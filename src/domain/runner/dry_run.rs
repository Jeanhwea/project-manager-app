use crate::utils::output::Output;
use anyhow::Result;
use std::path::Path;

use super::{CommandRunner, ExecutionContext, OutputMode};

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

    pub fn run_in_dir(&self, program: &str, args: &[&str], dir: Option<&Path>) -> Result<()> {
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
