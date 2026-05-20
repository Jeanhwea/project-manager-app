use colored::Colorize;
use std::path::Path;

const LABEL_WIDTH: usize = 8;
const HEADER_BAR_WIDTH: usize = 60;

mod symbols {
    pub const RESULT_PREFIX: &str = "<==";
    pub const COMMAND_PREFIX: &str = "==>";
    pub const DRY_COMMAND_PREFIX: &str = "~~>";
    pub const HEADER_PREFIX: &str = "===";
    pub const SECTION_PREFIX: &str = "##";
    pub const DRY_RUN_LABEL: &str = "[DRY]";
    pub const HEADER_FILL_CHAR: char = '=';
    pub const REMOVED_LINE_PREFIX: char = '-';
    pub const ADDED_LINE_PREFIX: char = '+';
}

#[derive(Clone, Copy)]
enum Color {
    Green,
    Red,
    Yellow,
    Cyan,
    Blue,
    Magenta,
    White,
}

#[derive(Clone, Copy, Default)]
struct Style {
    fg: Option<Color>,
    bold: bool,
    dim: bool,
    underline: bool,
}

impl Style {
    const fn plain() -> Self {
        Self {
            fg: None,
            bold: false,
            dim: false,
            underline: false,
        }
    }

    const fn fg(c: Color) -> Self {
        Self {
            fg: Some(c),
            ..Self::plain()
        }
    }

    const fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    const fn dim(mut self) -> Self {
        self.dim = true;
        self
    }

    const fn underline(mut self) -> Self {
        self.underline = true;
        self
    }
}

fn style_text(text: &str, style: Style) -> String {
    let mut s = text.normal();
    if let Some(c) = style.fg {
        s = match c {
            Color::Green => s.green(),
            Color::Red => s.red(),
            Color::Yellow => s.yellow(),
            Color::Cyan => s.cyan(),
            Color::Blue => s.blue(),
            Color::Magenta => s.magenta(),
            Color::White => s.white(),
        };
    }
    if style.bold {
        s = s.bold();
    }
    if style.dim {
        s = s.dimmed();
    }
    if style.underline {
        s = s.underline();
    }
    s.to_string()
}

fn print_line(s: &str) {
    println!("{}", s);
}

fn print_with_tag(tag: &str, tag_style: Style, body: &str, body_style: Style) {
    print_line(&format!(
        "{} {}",
        style_text(tag, tag_style),
        style_text(body, body_style)
    ));
}

const TAG_BLUE: Style = Style::fg(Color::Blue).bold();
const TAG_MAGENTA: Style = Style::fg(Color::Magenta).bold();
const TAG_GREEN: Style = Style::fg(Color::Green).bold();
const TAG_RED: Style = Style::fg(Color::Red).bold();
const TAG_YELLOW: Style = Style::fg(Color::Yellow).bold();
const TAG_CYAN: Style = Style::fg(Color::Cyan).bold();
const TAG_DIM: Style = Style::plain().dim();

const BODY_GREEN: Style = Style::fg(Color::Green);
const BODY_RED: Style = Style::fg(Color::Red);
const BODY_YELLOW: Style = Style::fg(Color::Yellow);
const BODY_DIM: Style = Style::plain().dim();

pub fn header(title: &str) {
    let head = format!("{} {} ", symbols::HEADER_PREFIX, title);
    let pad = HEADER_BAR_WIDTH.saturating_sub(display_width(&head));
    let fill: String = std::iter::repeat_n(symbols::HEADER_FILL_CHAR, pad).collect();
    print_line("");
    print_line(&format!(
        "{}{}",
        style_text(&head, TAG_GREEN),
        style_text(&fill, TAG_GREEN)
    ));
}

pub fn section(title: &str) {
    print_line("");
    print_line(&style_text(
        &format!("{} {}", symbols::SECTION_PREFIX, title),
        TAG_CYAN,
    ));
}

pub fn repo_header(index: usize, total: usize, path: &Path) {
    let total_str = total.to_string();
    let progress = format!("[{:>w$}/{}]", index, total, w = total_str.len());
    let progress = style_text(&progress, Style::fg(Color::White).bold());
    let path_str = style_text(
        &crate::utils::path::format_path(path),
        Style::fg(Color::Cyan).underline(),
    );
    print_line("");
    print_line(&format!("{} {}", progress, path_str));
}

pub fn command(cmd: &str) {
    let (_, body) = split_command_prefix(cmd);
    print_with_tag(symbols::COMMAND_PREFIX, TAG_BLUE, body, BODY_YELLOW);
}

pub fn dry_command(cmd: &str) {
    let (_, body) = split_command_prefix(cmd);
    print_with_tag(symbols::DRY_COMMAND_PREFIX, TAG_MAGENTA, body, BODY_YELLOW);
}

pub fn success(msg: &str) {
    print_with_tag(symbols::RESULT_PREFIX, TAG_GREEN, msg, BODY_GREEN);
}

pub fn error(msg: &str) {
    print_with_tag(symbols::RESULT_PREFIX, TAG_RED, msg, BODY_RED);
}

pub fn warning(msg: &str) {
    print_with_tag(symbols::RESULT_PREFIX, TAG_YELLOW, msg, BODY_YELLOW);
}

pub fn not_found(msg: &str) {
    warning(msg);
}

pub fn info(msg: &str) {
    print_line(&format!(
        "{} {}",
        style_text(symbols::RESULT_PREFIX, TAG_CYAN),
        msg
    ));
}

pub fn skip(msg: &str) {
    print_with_tag(symbols::RESULT_PREFIX, TAG_DIM, msg, BODY_DIM);
}

pub fn item(label: &str, value: &str) {
    let label = style_text(&pad_to_width(label, LABEL_WIDTH), TAG_GREEN);
    let value = style_text(value, BODY_YELLOW);
    print_line(&format!("{} {}", label, value));
}

pub fn detail(label: &str, value: &str) {
    let label = style_text(&pad_to_width(label, LABEL_WIDTH + 2), TAG_DIM);
    print_line(&format!("{} {}", label, value));
}

pub fn message(msg: &str) {
    print_line(msg);
}

pub fn blank() {
    print_line("");
}

pub fn removed_line(s: &str) {
    print_line(&format!(
        "{}{}",
        symbols::REMOVED_LINE_PREFIX,
        style_text(s, BODY_RED)
    ));
}

pub fn added_line(s: &str) {
    print_line(&format!(
        "{}{}",
        symbols::ADDED_LINE_PREFIX,
        style_text(s, BODY_GREEN)
    ));
}

pub fn dry_run_header(msg: &str) {
    print_line("");
    print_line(&format!(
        "{} {}",
        style_text(symbols::DRY_RUN_LABEL, TAG_MAGENTA),
        style_text(msg, TAG_CYAN),
    ));
}

fn split_command_prefix(cmd: &str) -> (Option<&str>, &str) {
    if let Some(rest) = cmd.strip_prefix('[')
        && let Some((dir, tail)) = rest.split_once(']')
        && let Some(body) = tail.strip_prefix(' ')
    {
        (Some(dir), body)
    } else {
        (None, cmd)
    }
}

fn display_width(s: &str) -> usize {
    s.chars()
        .map(|c| if (c as u32) < 0x80 { 1 } else { 2 })
        .sum()
}

fn pad_to_width(s: &str, width: usize) -> String {
    let w = display_width(s);
    if w >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - w))
    }
}
