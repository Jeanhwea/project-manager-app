use anyhow::{Context, Result};
use colored::*;
use std::path::Path;
use std::process::{Command, Output};

pub struct CommandRunner;

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
