use colored::Colorize;
use std::path::Path;

const SYMBOL_WIDTH: usize = 6;

fn tag_box(label: &str, color: colored::Color) -> String {
    format!("[{}]", label).color(color).bold().to_string()
}

pub trait OutputBackend {
    fn print(&self, msg: &str);
}

pub struct TerminalBackend;

impl OutputBackend for TerminalBackend {
    fn print(&self, msg: &str) {
        println!("{}", msg);
    }
}

pub struct Output;

impl Output {
    pub fn header(title: &str) {
        let backend = TerminalBackend;
        backend.print("");
        let line = format!("== {} ", title);
        let padding = 60usize.saturating_sub(line.len());
        backend.print(&format!(
            "{}{}",
            line.green().bold(),
            "=".repeat(padding).green().bold()
        ));
    }

    pub fn section(title: &str) {
        let backend = TerminalBackend;
        backend.print("");
        backend.print(&format!("--- {} ---", title).cyan().bold().to_string());
    }

    pub fn repo_header(index: usize, total: usize, path: &Path) {
        let backend = TerminalBackend;
        backend.print("");
        let progress = format!("[{}/{}]", index, total);
        let path_str = crate::utils::path::format_path(path);
        backend.print(&format!(
            "{} {}",
            progress.white().bold(),
            path_str.cyan().underline()
        ));
    }

    pub fn success(msg: &str) {
        let backend = TerminalBackend;
        backend.print(&format!("{} {}", "<==".green().bold(), msg.green()));
    }

    pub fn error(msg: &str) {
        let backend = TerminalBackend;
        backend.print(&format!("{} {}", "<==".red().bold(), msg.red()));
    }

    pub fn warning(msg: &str) {
        let backend = TerminalBackend;
        backend.print(&format!("{} {}", "<==".yellow().bold(), msg.yellow()));
    }

    pub fn info(msg: &str) {
        let backend = TerminalBackend;
        backend.print(&format!("{} {}", "<==".cyan().bold(), msg));
    }

    pub fn skip(msg: &str) {
        let backend = TerminalBackend;
        backend.print(&format!("{} {}", "<==".dimmed(), msg.dimmed()));
    }

    pub fn cmd(cmd: &str) {
        let backend = TerminalBackend;
        backend.print(&format!("{} {}", "==>".blue().bold(), cmd.yellow()));
    }

    pub fn item(label: &str, value: &str) {
        let backend = TerminalBackend;
        let padded = format!("{:<width$}", label, width = SYMBOL_WIDTH);
        backend.print(&format!("{} {}", padded.green().bold(), value.yellow()));
    }

    pub fn detail(label: &str, value: &str) {
        let backend = TerminalBackend;
        let padded = format!("{:<width$}", label, width = SYMBOL_WIDTH + 2);
        backend.print(&format!("{} {}", padded.dimmed(), value));
    }

    pub fn message(msg: &str) {
        let backend = TerminalBackend;
        backend.print(msg);
    }

    pub fn blank() {
        let backend = TerminalBackend;
        backend.print("");
    }

    pub fn dry_run_header(msg: &str) {
        let backend = TerminalBackend;
        backend.print("");
        let tag = tag_box("DRY", colored::Color::Magenta);
        backend.print(&format!("{} {}", tag, msg.cyan().bold()));
    }

    pub fn not_found(msg: &str) {
        let backend = TerminalBackend;
        backend.print(&format!("{} {}", "<==".yellow().bold(), msg.yellow()));
    }
}
