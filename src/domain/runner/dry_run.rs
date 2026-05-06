use crate::utils::output::Output;
use anyhow::Result;
use std::path::Path;
use std::sync::Arc;

use super::{CommandRunner, DefaultCommandRunner, ExecutionContext, OutputMode};

pub struct DryRunContext {
    dry_run: bool,
    runner: Arc<dyn CommandRunner>,
}

impl DryRunContext {
    pub fn new(dry_run: bool) -> Self {
        Self {
            dry_run,
            runner: Arc::new(DefaultCommandRunner),
        }
    }

    pub fn is_dry_run(&self) -> bool {
        self.dry_run
    }

    pub fn run_in_dir(&self, program: &str, args: &[&str], dir: Option<&Path>) -> Result<()> {
        // 根据 dry_run 标志选择输出模式
        let mode = if self.dry_run {
            OutputMode::DryRun
        } else {
            OutputMode::Streaming // 实际执行时使用流式输出
        };

        // 使用 ExecutionContext 构建命令上下文
        let mut ctx = ExecutionContext::new(program)
            .args(args.iter().map(|s| *s))
            .output_mode(mode);

        // 如果提供了工作目录，设置工作目录
        if let Some(dir) = dir {
            ctx = ctx.working_dir(dir);
        }

        // 执行命令
        let result = self
            .runner
            .execute(&ctx)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // 检查执行结果
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
