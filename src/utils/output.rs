use colored::Colorize;
use std::path::Path;

const LABEL_WIDTH: usize = 8;
const HEADER_WIDTH: usize = 60;

const ICON_RUN: &str = "→";
const ICON_OK: &str = "✓";
const ICON_FAIL: &str = "✗";
const ICON_WARN: &str = "⚠";
const ICON_INFO: &str = "ℹ";
const ICON_SKIP: &str = "·";
const ICON_DRY: &str = "~";

pub struct Output;

impl Output {
    fn print(msg: &str) {
        println!("{}", msg);
    }

    pub fn header(title: &str) {
        let line = format!("═══ {} ", title);
        let pad = HEADER_WIDTH.saturating_sub(display_width(&line));
        let fill = "═".repeat(pad);
        Self::print("");
        Self::print(&format!("{}{}", line.green().bold(), fill.green().bold()));
    }

    pub fn section(title: &str) {
        Self::print("");
        Self::print(&format!("▎ {}", title).cyan().bold().to_string());
    }

    pub fn repo_header(index: usize, total: usize, path: &Path) {
        let total_str = total.to_string();
        let progress = format!("[{:>width$}/{}]", index, total, width = total_str.len())
            .white()
            .bold()
            .to_string();
        let path_str = crate::utils::path::format_path(path)
            .cyan()
            .underline()
            .to_string();
        Self::print("");
        Self::print(&format!("{} {}", progress, path_str));
    }

    pub fn cmd(cmd: &str) {
        let (dir, command) = split_cmd(cmd);
        let icon = ICON_RUN.blue().bold().to_string();
        match dir {
            Some(d) => Self::print(&format!(
                "{} {} {}",
                icon,
                format!("[{}]", d).dimmed(),
                command.yellow()
            )),
            None => Self::print(&format!("{} {}", icon, command.yellow())),
        }
    }

    pub fn dry_cmd(cmd: &str) {
        let (dir, command) = split_cmd(cmd);
        let icon = ICON_DRY.magenta().bold().to_string();
        match dir {
            Some(d) => Self::print(&format!(
                "{} {} {}",
                icon,
                format!("[{}]", d).dimmed(),
                command.yellow()
            )),
            None => Self::print(&format!("{} {}", icon, command.yellow())),
        }
    }

    pub fn success(msg: &str) {
        Self::print(&format!("{} {}", ICON_OK.green().bold(), msg.green()));
    }

    pub fn error(msg: &str) {
        Self::print(&format!("{} {}", ICON_FAIL.red().bold(), msg.red()));
    }

    pub fn warning(msg: &str) {
        Self::print(&format!("{} {}", ICON_WARN.yellow().bold(), msg.yellow()));
    }

    pub fn not_found(msg: &str) {
        Self::warning(msg);
    }

    pub fn info(msg: &str) {
        Self::print(&format!("{} {}", ICON_INFO.cyan().bold(), msg));
    }

    pub fn skip(msg: &str) {
        Self::print(&format!("{} {}", ICON_SKIP.dimmed(), msg.dimmed()));
    }

    pub fn item(label: &str, value: &str) {
        let padded = pad_display(label, LABEL_WIDTH).green().bold().to_string();
        Self::print(&format!("{} {}", padded, value.yellow()));
    }

    pub fn detail(label: &str, value: &str) {
        let padded = pad_display(label, LABEL_WIDTH + 2).dimmed().to_string();
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
        let tag = " DRY-RUN ".black().on_magenta().bold().to_string();
        Self::print("");
        Self::print(&format!("{} {}", tag, msg.cyan().bold()));
    }
}

fn split_cmd(cmd: &str) -> (Option<&str>, &str) {
    if let Some(rest) = cmd.strip_prefix('[')
        && let Some((dir, tail)) = rest.split_once(']')
        && let Some(command) = tail.strip_prefix(' ')
    {
        (Some(dir), command)
    } else {
        (None, cmd)
    }
}

fn display_width(s: &str) -> usize {
    s.chars()
        .map(|c| if (c as u32) < 0x80 { 1 } else { 2 })
        .sum()
}

fn pad_display(s: &str, width: usize) -> String {
    let w = display_width(s);
    if w >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - w))
    }
}
