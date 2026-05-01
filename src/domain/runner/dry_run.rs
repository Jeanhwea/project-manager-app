//! Dry run context for previewing operations without executing them.
//!
//! Provides a shared `DryRunContext` used across all commands that support `--dry-run`.

use crate::domain::git::command::GitCommandRunner;
use anyhow::Result;
use colored::Colorize;
use std::path::Path;

/// Context for dry-run mode that previews operations instead of executing them.
///
/// When `dry_run` is true, commands are printed but not executed.
/// When `dry_run` is false, commands are executed normally via `GitCommandRunner`.
pub struct DryRunContext {
    dry_run: bool,
}

impl DryRunContext {
    /// Create a new dry-run context.
    pub fn new(dry_run: bool) -> Self {
        Self { dry_run }
    }

    /// Check if dry-run mode is enabled.
    pub fn is_dry_run(&self) -> bool {
        self.dry_run
    }

    /// Run a git command, or print it in dry-run mode.
    ///
    /// When `dir` is `Some`, the command runs in that directory.
    pub fn run_in_dir(&self, program: &str, args: &[&str], dir: Option<&Path>) -> Result<()> {
        if self.dry_run {
            self.print_dry_run_command(program, args, dir);
            return Ok(());
        }

        let runner = GitCommandRunner::new();
        if let Some(dir) = dir {
            runner
                .execute_with_success_in_dir(args, dir)
                .map_err(|e| anyhow::anyhow!("{}", e))
        } else {
            runner
                .execute_with_success(args)
                .map_err(|e| anyhow::anyhow!("{}", e))
        }
    }

    /// Print a section header (only in dry-run mode).
    pub fn print_header(&self, msg: &str) {
        if self.dry_run {
            println!("{}", msg.green().bold());
        }
    }

    /// Print an informational message (only in dry-run mode).
    pub fn print_message(&self, msg: &str) {
        if self.dry_run {
            println!("  {} {}", "[DRY-RUN]".yellow(), msg);
        }
    }

    /// Print a file diff showing before/after changes (only in dry-run mode).
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

    fn print_dry_run_command(&self, program: &str, args: &[&str], dir: Option<&Path>) {
        let args_str = args.join(" ");
        if let Some(d) = dir {
            println!(
                "  {} {} {} (in {})",
                "[DRY-RUN]".yellow(),
                program.cyan(),
                args_str,
                d.display().to_string().dimmed()
            );
        } else {
            println!("  {} {} {}", "[DRY-RUN]".yellow(), program.cyan(), args_str);
        }
    }
}
