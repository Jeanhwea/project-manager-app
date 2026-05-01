use crate::domain::git::command::GitCommandRunner;
use anyhow::Result;
use colored::Colorize;
use std::path::Path;

/// 支持 `--dry-run` 的命令执行上下文。
///
/// dry_run=true 时只打印将要执行的命令，不实际执行。
/// dry_run=false 时打印命令、执行、并输出 stdout/stderr。
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

    /// 执行命令。dry-run 模式下只打印，否则打印并执行。
    pub fn run_in_dir(&self, program: &str, args: &[&str], dir: Option<&Path>) -> Result<()> {
        if self.dry_run {
            self.print_dry_run_command(program, args);
            return Ok(());
        }

        // 打印即将执行的命令
        println!(
            "{} {} {}",
            "=>".white(),
            program.yellow(),
            args.join(" ").yellow()
        );

        let runner = GitCommandRunner::new();
        let output = if let Some(dir) = dir {
            runner.execute_raw_in_dir(args, dir)
        } else {
            runner.execute_raw(args)
        }
        .map_err(|e| anyhow::anyhow!("{}", e))?;

        // 打印 stdout/stderr
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
            println!("{}", msg.green().bold());
        }
    }

    pub fn print_message(&self, msg: &str) {
        if self.dry_run {
            println!("  {} {}", "[DRY-RUN]".yellow(), msg);
        }
    }

    pub fn print_file_diff(&self, file_path: &str, old_content: &str, new_content: &str) {
        if !self.dry_run {
            return;
        }

        println!("\n  {}", file_path.blue().underline());

        let old_lines: Vec<&str> = old_content.lines().collect();
        let new_lines: Vec<&str> = new_content.lines().collect();

        for (line_num, (old_line, new_line)) in (1..).zip(old_lines.iter().zip(new_lines.iter()))
        {
            if old_line != new_line {
                println!("    {} {}", format!("L{}:", line_num).dimmed(), "-".red());
                println!("      {}", old_line.red());
                println!("    {} {}", format!("L{}:", line_num).dimmed(), "+".green());
                println!("      {}", new_line.green());
            }
        }
    }

    fn print_dry_run_command(&self, program: &str, args: &[&str]) {
        println!(
            "  {} {} {}",
            "[DRY-RUN]".yellow(),
            program.cyan(),
            args.join(" ")
        );
    }
}
