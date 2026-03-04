use colored::*;
use std::process::{Command, Output};

pub struct CommandRunner;

impl CommandRunner {
    pub fn run(program: &str, args: &[&str]) -> Result<Output, String> {
        Self::run_internal(program, args, true)
    }

    pub fn run_quiet(program: &str, args: &[&str]) -> Result<Output, String> {
        Self::run_internal(program, args, false)
    }

    fn run_internal(program: &str, args: &[&str], verbose: bool) -> Result<Output, String> {
        let mut cmd = Command::new(program);
        cmd.args(args);

        if verbose {
            Self::print_command(&cmd);
        }

        let output = cmd
            .output()
            .map_err(|e| format!("执行 {} 失败: {}", program, e))?;

        if verbose {
            Self::print_output(&output);
        }

        Ok(output)
    }

    pub fn run_with_success(program: &str, args: &[&str]) -> Result<Output, String> {
        let output = Self::run(program, args)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("命令执行失败: {}", stderr));
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
            println!("{}", stdout);
        }

        if !stderr.is_empty() {
            eprintln!("{}", stderr.white());
        }
    }
}
