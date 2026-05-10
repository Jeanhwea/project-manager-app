use colored::Colorize;
use std::path::Path;

const SYMBOL_WIDTH: usize = 6;

fn tag_box(label: &str, color: colored::Color) -> String {
    format!("[{}]", label).color(color).bold().to_string()
}

pub struct Output;

impl Output {
    pub fn header(title: &str) {
        println!();
        let line = format!("== {} ", title);
        let padding = 60usize.saturating_sub(line.len());
        println!(
            "{}{}",
            line.green().bold(),
            "=".repeat(padding).green().bold()
        );
    }

    pub fn section(title: &str) {
        println!();
        println!("{}", format!("--- {} ---", title).cyan().bold());
    }

    pub fn repo_header(index: usize, total: usize, path: &Path) {
        println!();
        let progress = format!("[{}/{}]", index, total);
        let path_str = crate::utils::path::format_path(path);
        println!(
            "{} {}",
            progress.white().bold(),
            path_str.cyan().underline()
        );
    }

    pub fn success(msg: &str) {
        println!("{} {}", "<==".green().bold(), msg.green());
    }

    pub fn error(msg: &str) {
        println!("{} {}", "<==".red().bold(), msg.red());
    }

    pub fn warning(msg: &str) {
        println!("{} {}", "<==".yellow().bold(), msg.yellow());
    }

    pub fn info(msg: &str) {
        println!("{} {}", "<==".cyan().bold(), msg);
    }

    pub fn skip(msg: &str) {
        println!("{} {}", "<==".dimmed(), msg.dimmed());
    }

    pub fn cmd(cmd: &str) {
        println!("{} {}", "==>".blue().bold(), cmd.yellow());
    }

    pub fn item(label: &str, value: &str) {
        let padded = format!("{:<width$}", label, width = SYMBOL_WIDTH);
        println!("{} {}", padded.green().bold(), value.yellow());
    }

    pub fn item_colored(label: &str, value: &str, color: ItemColor) {
        let padded = format!("{:<width$}", label, width = SYMBOL_WIDTH);
        let colored_value = match color {
            ItemColor::Green => value.green(),
            ItemColor::Yellow => value.yellow(),
            ItemColor::Red => value.red(),
            ItemColor::Cyan => value.cyan(),
        };
        println!("{} {}", padded.green().bold(), colored_value);
    }

    pub fn detail(label: &str, value: &str) {
        let padded = format!("{:<width$}", label, width = SYMBOL_WIDTH + 2);
        println!("{} {}", padded.dimmed(), value);
    }

    pub fn message(msg: &str) {
        println!("{}", msg);
    }

    pub fn blank() {
        println!();
    }

    pub fn dry_run_header(msg: &str) {
        println!();
        let tag = tag_box("DRY", colored::Color::Magenta);
        println!("{} {}", tag, msg.cyan().bold());
    }

    pub fn not_found(msg: &str) {
        println!("{} {}", "<==".yellow().bold(), msg.yellow());
    }
}

#[allow(dead_code)]
pub enum ItemColor {
    Green,
    Yellow,
    Red,
    Cyan,
}
