use crate::domain::git::command::GitCommandRunner;
use crate::utils::output::Output;
use anyhow::Result;
use std::path::Path;

/// 支持 `--dry-run` 的命令执行上下文
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

    /// 执行命令。dry-run 模式下只打印，否则打印并执行
    pub fn run_in_dir(&self, program: &str, args: &[&str], dir: Option<&Path>) -> Result<()> {
        if self.dry_run {
            self.print_dry_run_command(program, args);
            return Ok(());
        }

        Output::info(&format!("{} {}", program, args.join(" ")));

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

    #[allow(dead_code)]
    pub fn print_file_diff(&self, file_path: &str, old_content: &str, new_content: &str) {
        if !self.dry_run {
            return;
        }

        Output::blank();
        Output::message(&format!("File: {}", file_path));

        let old_lines: Vec<&str> = old_content.lines().collect();
        let new_lines: Vec<&str> = new_content.lines().collect();

        for (line_num, (old_line, new_line)) in (1..).zip(old_lines.iter().zip(new_lines.iter())) {
            if old_line != new_line {
                Output::detail(&format!("L{} -", line_num), old_line);
                Output::detail(&format!("L{} +", line_num), new_line);
            }
        }
    }

    fn print_dry_run_command(&self, program: &str, args: &[&str]) {
        Output::message(&format!("[DRY-RUN] {} {}", program, args.join(" ")));
    }
}
