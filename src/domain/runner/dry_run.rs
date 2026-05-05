use crate::domain::git::command::GitCommandRunner;
use crate::utils::output::Output;
use anyhow::Result;
use std::path::Path;

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
        if self.dry_run {
            self.print_dry_run_command(program, args);
            return Ok(());
        }

        Output::cmd(&format!("{} {}", program, args.join(" ")));

        let runner = GitCommandRunner::new();
        let output = if let Some(dir) = dir {
            runner.execute_raw_in_dir(args, dir)
        } else {
            runner.execute_raw(args)
        }
        .map_err(|e| anyhow::anyhow!("{}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stdout.is_empty() {
            print!("{}", stdout);
        }
        if !stderr.is_empty() {
            eprint!("{}", stderr);
        }

        if !output.status.success() {
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

    fn print_dry_run_command(&self, program: &str, args: &[&str]) {
        Output::cmd(&format!("{} {}", program, args.join(" ")));
    }
}
