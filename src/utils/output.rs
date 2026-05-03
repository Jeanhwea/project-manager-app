use colored::Colorize;
use std::path::Path;

/// ASCII-only symbols for terminal output
const SYMBOL_SUCCESS: &str = "OK>";
const SYMBOL_ERROR: &str = "FAIL";
const SYMBOL_WARNING: &str = "WARN";
const SYMBOL_INFO: &str = "INFO";
const SYMBOL_SKIP: &str = "SKIP";
const SYMBOL_CMD: &str = "==>";

/// Alignment width for all symbols
const SYMBOL_WIDTH: usize = 4;

pub struct Output;

impl Output {
    pub fn header(title: &str) {
        println!();
        println!("{}", format!("-- {} --", title).green().bold());
    }

    pub fn section(title: &str) {
        println!();
        println!("{}", title.cyan().bold());
    }

    pub fn repo_header(index: usize, total: usize, path: &Path) {
        let progress = format!("({}/{})", index, total);
        println!(
            "{}>> {}",
            progress.white().bold(),
            crate::utils::path::format_path(path).cyan().underline()
        );
    }

    pub fn success(msg: &str) {
        let symbol = format!("{:<width$}", SYMBOL_SUCCESS, width = SYMBOL_WIDTH);
        println!("  {} {}", symbol.green(), msg.green());
    }

    pub fn error(msg: &str) {
        println!("  {} {}", SYMBOL_ERROR.red(), msg.red());
    }

    pub fn warning(msg: &str) {
        println!("  {} {}", SYMBOL_WARNING.yellow(), msg.yellow());
    }

    pub fn info(msg: &str) {
        println!("  {} {}", SYMBOL_INFO.cyan(), msg);
    }

    pub fn skip(msg: &str) {
        println!("  {} {}", SYMBOL_SKIP.dimmed(), msg.dimmed());
    }

    /// Display a command that will be executed.
    /// Used for dry-run previews and actual execution confirmation.
    pub fn cmd(cmd: &str) {
        println!("  {} {}", SYMBOL_CMD.cyan(), cmd);
    }

    pub fn item(label: &str, value: &str) {
        println!("  {}: {}", label, value.yellow());
    }

    pub fn item_colored(label: &str, value: &str, color: ItemColor) {
        let colored_value = match color {
            ItemColor::Green => value.green(),
            ItemColor::Yellow => value.yellow(),
            ItemColor::Red => value.red(),
            ItemColor::Cyan => value.cyan(),
            ItemColor::Dimmed => value.dimmed(),
        };
        println!("  {}: {}", label, colored_value);
    }

    pub fn detail(label: &str, value: &str) {
        println!("    {}: {}", label.dimmed(), value);
    }

    pub fn message(msg: &str) {
        println!("  {}", msg);
    }

    pub fn blank() {
        println!();
    }

    pub fn dry_run_header(msg: &str) {
        println!();
        println!("{}", format!("[DRY-RUN] {}", msg).cyan().bold());
    }

    pub fn not_found(msg: &str) {
        println!("{}", msg.yellow());
    }
}

#[allow(dead_code)]
pub enum ItemColor {
    Green,
    Yellow,
    Red,
    Cyan,
    Dimmed,
}

pub struct SummaryBuilder {
    items: Vec<(String, String)>,
}

impl SummaryBuilder {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn add(mut self, label: &str, value: impl Into<String>) -> Self {
        self.items.push((label.to_string(), value.into()));
        self
    }

    pub fn print(self) {
        for (label, value) in self.items {
            println!("  {}: {}", label, value);
        }
    }
}

impl Default for SummaryBuilder {
    fn default() -> Self {
        Self::new()
    }
}
