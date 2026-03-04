use std::process::{Command, Output};
use colored::*;

pub struct CommandRunner;

impl CommandRunner {
    pub fn run(program: &str, args: &[&str]) -> Result<Output, String> {
        let mut cmd = Command::new(program);
        cmd.args(args);
        Self::print_command(&cmd);

        let output = cmd
            .output()
            .map_err(|e| format!("执行 {} 失败: {}", program, e))?;

        Self::print_output(&output);

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
            "==>".cyan(),
            cmd.get_program().to_string_lossy().bright_blue(),
            args.join(" ").bright_blue()
        );
    }

    fn print_output(output: &Output) {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stdout.is_empty() {
            println!("{} {}", "<==".green(), stdout.bright_white());
        }
        if !stderr.is_empty() {
            eprintln!("{} {}", "!!!".red(), stderr.bright_red());
        }
    }
}
