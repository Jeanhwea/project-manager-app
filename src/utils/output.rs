use colored::Colorize;
use std::path::Path;

const LABEL_WIDTH: usize = 6;
const HEADER_WIDTH: usize = 60;
const ARROW_IN: &str = "<==";
const ARROW_OUT: &str = "==>";

pub struct Output;

impl Output {
    fn print(msg: &str) {
        println!("{}", msg);
    }

    pub fn header(title: &str) {
        let line = format!("== {} ", title);
        let fill = "=".repeat(HEADER_WIDTH.saturating_sub(line.len()));
        Self::print("");
        Self::print(&format!("{}{}", line.green().bold(), fill.green().bold()));
    }

    pub fn section(title: &str) {
        Self::print("");
        Self::print(&format!("--- {} ---", title).cyan().bold().to_string());
    }

    pub fn repo_header(index: usize, total: usize, path: &Path) {
        let progress = format!("[{}/{}]", index, total).white().bold().to_string();
        let path_str = crate::utils::path::format_path(path)
            .cyan()
            .underline()
            .to_string();
        Self::print("");
        Self::print(&format!("{} {}", progress, path_str));
    }

    fn strip_dir(cmd: &str) -> &str {
        if let Some((_, rest)) = cmd.split_once(']')
            && let Some(command) = rest.strip_prefix(' ')
        {
            command
        } else {
            cmd
        }
    }

    pub fn cmd(cmd: &str) {
        let command = Self::strip_dir(cmd);
        Self::print(&format!("{} {}", ARROW_OUT.blue().bold(), command.yellow()));
    }

    pub fn dry_cmd(cmd: &str) {
        let arrow = ARROW_OUT.blue().bold().to_string();
        let body = if let Some((dir, rest)) = cmd.split_once(']')
            && let Some(command) = rest.strip_prefix(' ')
        {
            format!("{} {}", format!("{}]", dir).blue().bold(), command.yellow())
        } else {
            cmd.yellow().to_string()
        };
        Self::print(&format!("{} {}", arrow, body));
    }

    pub fn success(msg: &str) {
        Self::print(&format!("{} {}", ARROW_IN.green().bold(), msg.green()));
    }

    pub fn error(msg: &str) {
        Self::print(&format!("{} {}", ARROW_IN.red().bold(), msg.red()));
    }

    pub fn warning(msg: &str) {
        Self::print(&format!("{} {}", ARROW_IN.yellow().bold(), msg.yellow()));
    }

    pub fn not_found(msg: &str) {
        Self::warning(msg);
    }

    pub fn info(msg: &str) {
        Self::print(&format!("{} {}", ARROW_IN.cyan().bold(), msg));
    }

    pub fn skip(msg: &str) {
        Self::print(&format!("{} {}", ARROW_IN.dimmed(), msg.dimmed()));
    }

    pub fn item(label: &str, value: &str) {
        let padded = format!("{:<width$}", label, width = LABEL_WIDTH)
            .green()
            .bold()
            .to_string();
        Self::print(&format!("{} {}", padded, value.yellow()));
    }

    pub fn detail(label: &str, value: &str) {
        let padded = format!("{:<width$}", label, width = LABEL_WIDTH + 2)
            .dimmed()
            .to_string();
        Self::print(&format!("{} {}", padded, value));
    }

    pub fn message(msg: &str) {
        Self::print(msg);
    }

    pub fn blank() {
        Self::print("");
    }

    pub fn diff_old(line: &str) {
        Self::print(&format!("-{}", line.red()));
    }

    pub fn diff_new(line: &str) {
        Self::print(&format!("+{}", line.green()));
    }

    pub fn dry_run_header(msg: &str) {
        let tag = "[DRY]".magenta().bold().to_string();
        Self::print("");
        Self::print(&format!("{} {}", tag, msg.cyan().bold()));
    }
}
