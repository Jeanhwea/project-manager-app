use anyhow::{Context, Result};
use colored::*;
use std::path::Path;
use std::process::{Command, Output};

#[cfg(windows)]
use std::os::windows::process::ExitStatusExt;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

pub struct CommandRunner;

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

    #[allow(dead_code)]
    pub fn run(&self, program: &str, args: &[&str]) -> Result<()> {
        self.run_in_dir(program, args, None)
    }

    pub fn run_in_dir(&self, program: &str, args: &[&str], dir: Option<&Path>) -> Result<()> {
        if self.dry_run {
            self.print_dry_run_command(program, args, dir);
            return Ok(());
        }

        if let Some(d) = dir {
            CommandRunner::run_with_success_in_dir(program, args, d)?;
        } else {
            CommandRunner::run_with_success(program, args)?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn run_quiet_in_dir(&self, program: &str, args: &[&str], dir: &Path) -> Result<Output> {
        if self.dry_run {
            self.print_dry_run_command(program, args, Some(dir));
            return Ok(Output {
                status: std::process::ExitStatus::from_raw(0),
                stdout: Vec::new(),
                stderr: Vec::new(),
            });
        }
        CommandRunner::run_quiet_in_dir(program, args, dir)
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

    pub fn print_message(&self, msg: &str) {
        if self.dry_run {
            println!("  {} {}", "[DRY-RUN]".yellow(), msg);
        }
    }

    pub fn print_header(&self, msg: &str) {
        if self.dry_run {
            println!("{}", msg.green().bold());
        }
    }
}

impl CommandRunner {
    pub fn run_quiet(program: &str, args: &[&str]) -> Result<Output> {
        Self::execute(program, args, None, false)
    }

    pub fn run_with_success(program: &str, args: &[&str]) -> Result<Output> {
        Self::execute_checked(program, args, None)
    }

    pub fn run_quiet_in_dir(program: &str, args: &[&str], dir: &Path) -> Result<Output> {
        Self::execute(program, args, Some(dir), false)
    }

    pub fn run_with_success_in_dir(program: &str, args: &[&str], dir: &Path) -> Result<Output> {
        Self::execute_checked(program, args, Some(dir))
    }

    fn execute(
        program: &str,
        args: &[&str],
        dir: Option<&Path>,
        verbose: bool,
    ) -> Result<Output> {
        let mut cmd = Command::new(program);
        cmd.args(args);
        if let Some(dir) = dir {
            cmd.current_dir(dir);
        }

        if verbose {
            Self::print_command(&cmd);
        }

        let output = cmd
            .output()
            .with_context(|| format!("执行 {} 失败", program))?;

        if verbose {
            Self::print_output(&output);
        }

        Ok(output)
    }

    fn execute_checked(program: &str, args: &[&str], dir: Option<&Path>) -> Result<Output> {
        let output = Self::execute(program, args, dir, true)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("命令执行失败: {}", stderr);
        }
        Ok(output)
    }

    fn print_command(cmd: &Command) {
        let args: Vec<String> = cmd
            .get_args()
            .map(|s| s.to_string_lossy().to_string())
            .collect();
        println!(
            "{} {} {}",
            "=>".white(),
            cmd.get_program().to_string_lossy().yellow(),
            args.join(" ").yellow()
        );
    }

    fn print_output(output: &Output) {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !stdout.is_empty() {
            print!("{}", stdout);
        }

        if !stderr.is_empty() {
            eprint!("{}", stderr.white());
        }
    }
}
