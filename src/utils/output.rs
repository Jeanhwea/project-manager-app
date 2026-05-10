use colored::Colorize;
use std::path::Path;

const SYMBOL_WIDTH: usize = 6;

const TAG_OK: &str = "  OK  ";
const TAG_FAIL: &str = " FAIL ";
const TAG_WARN: &str = " WARN ";
const TAG_INFO: &str = " INFO ";
const TAG_SKIP: &str = " SKIP ";
const TAG_CMD: &str = " EXEC ";
const TAG_DRY: &str = " DRY  ";

fn tag_box(label: &str, color: colored::Color) -> String {
    format!("[{}]", label)
        .color(color)
        .bold()
        .to_string()
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
        let tag = tag_box(TAG_OK, colored::Color::Green);
        println!("  {} {}", tag, msg.green());
    }

    pub fn error(msg: &str) {
        let tag = tag_box(TAG_FAIL, colored::Color::Red);
        println!("  {} {}", tag, msg.red());
    }

    pub fn warning(msg: &str) {
        let tag = tag_box(TAG_WARN, colored::Color::Yellow);
        println!("  {} {}", tag, msg.yellow());
    }

    pub fn info(msg: &str) {
        let tag = tag_box(TAG_INFO, colored::Color::Cyan);
        println!("  {} {}", tag, msg);
    }

    pub fn skip(msg: &str) {
        let tag = tag_box(TAG_SKIP, colored::Color::White);
        println!("  {} {}", tag.dimmed(), msg.dimmed());
    }

    pub fn cmd(cmd: &str) {
        let tag = tag_box(TAG_CMD, colored::Color::Cyan);
        println!("  {} {}", tag, cmd.white());
    }

    pub fn item(label: &str, value: &str) {
        let padded = format!("{:<width$}", label, width = SYMBOL_WIDTH);
        println!("  {} {}", padded.green().bold(), value.yellow());
    }

    pub fn item_colored(label: &str, value: &str, color: ItemColor) {
        let padded = format!("{:<width$}", label, width = SYMBOL_WIDTH);
        let colored_value = match color {
            ItemColor::Green => value.green(),
            ItemColor::Yellow => value.yellow(),
            ItemColor::Red => value.red(),
            ItemColor::Cyan => value.cyan(),
        };
        println!("  {} {}", padded.green().bold(), colored_value);
    }

    pub fn detail(label: &str, value: &str) {
        let padded = format!("{:<width$}", label, width = SYMBOL_WIDTH + 2);
        println!("  {} {}", padded.dimmed(), value);
    }

    pub fn message(msg: &str) {
        println!("  {}", msg);
    }

    pub fn blank() {
        println!();
    }

    pub fn dry_run_header(msg: &str) {
        println!();
        let tag = tag_box(TAG_DRY, colored::Color::Magenta);
        println!("{} {}", tag, msg.cyan().bold());
    }

    pub fn not_found(msg: &str) {
        let tag = tag_box(TAG_WARN, colored::Color::Yellow);
        println!("  {} {}", tag, msg.yellow());
    }
}

#[allow(dead_code)]
pub enum ItemColor {
    Green,
    Yellow,
    Red,
    Cyan,
}
