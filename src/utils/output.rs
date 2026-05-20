use colored::Colorize;
use std::path::Path;

const LABEL_WIDTH: usize = 8;
const HEADER_BAR_WIDTH: usize = 60;

mod glyph {
    pub const ARROW_IN: &str = "<==";
    pub const ARROW_OUT: &str = "==>";
    pub const ARROW_DRY: &str = "~~>";
    pub const HEADER_PREFIX: &str = "===";
    pub const SECTION_PREFIX: &str = "##";
    pub const DRY_TAG: &str = "[DRY]";
    pub const HEADER_FILL: char = '=';
    pub const DIFF_OLD: char = '-';
    pub const DIFF_NEW: char = '+';
}

#[derive(Clone, Copy)]
enum Tone {
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
    fg: Option<Tone>,
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

    const fn fg(t: Tone) -> Self {
        Self {
            fg: Some(t),
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

fn paint(text: &str, style: Style) -> String {
    let mut s = text.normal();
    if let Some(t) = style.fg {
        s = match t {
            Tone::Green => s.green(),
            Tone::Red => s.red(),
            Tone::Yellow => s.yellow(),
            Tone::Cyan => s.cyan(),
            Tone::Blue => s.blue(),
            Tone::Magenta => s.magenta(),
            Tone::White => s.white(),
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

fn line(s: &str) {
    println!("{}", s);
}

fn tagged(tag: &str, tag_style: Style, body: &str, body_style: Style) {
    line(&format!("{} {}", paint(tag, tag_style), paint(body, body_style)));
}

// === Style presets ===

const TAG_BLUE: Style = Style::fg(Tone::Blue).bold();
const TAG_MAGENTA: Style = Style::fg(Tone::Magenta).bold();
const TAG_GREEN: Style = Style::fg(Tone::Green).bold();
const TAG_RED: Style = Style::fg(Tone::Red).bold();
const TAG_YELLOW: Style = Style::fg(Tone::Yellow).bold();
const TAG_CYAN: Style = Style::fg(Tone::Cyan).bold();
const TAG_DIM: Style = Style::plain().dim();

const BODY_GREEN: Style = Style::fg(Tone::Green);
const BODY_RED: Style = Style::fg(Tone::Red);
const BODY_YELLOW: Style = Style::fg(Tone::Yellow);
const BODY_DIM: Style = Style::plain().dim();

// === Public API ===

pub fn header(title: &str) {
    let head = format!("{} {} ", glyph::HEADER_PREFIX, title);
    let pad = HEADER_BAR_WIDTH.saturating_sub(display_width(&head));
    let fill: String = std::iter::repeat(glyph::HEADER_FILL).take(pad).collect();
    line("");
    line(&format!(
        "{}{}",
        paint(&head, TAG_GREEN),
        paint(&fill, TAG_GREEN)
    ));
}

pub fn section(title: &str) {
    line("");
    line(&paint(
        &format!("{} {}", glyph::SECTION_PREFIX, title),
        TAG_CYAN,
    ));
}

pub fn repo_header(index: usize, total: usize, path: &Path) {
    let total_str = total.to_string();
    let progress = format!("[{:>w$}/{}]", index, total, w = total_str.len());
    let progress = paint(&progress, Style::fg(Tone::White).bold());
    let path_str = paint(
        &crate::utils::path::format_path(path),
        Style::fg(Tone::Cyan).underline(),
    );
    line("");
    line(&format!("{} {}", progress, path_str));
}

pub fn cmd(cmd: &str) {
    let (_, body) = split_cmd(cmd);
    tagged(glyph::ARROW_OUT, TAG_BLUE, body, BODY_YELLOW);
}

pub fn dry_cmd(cmd: &str) {
    let (_, body) = split_cmd(cmd);
    tagged(glyph::ARROW_DRY, TAG_MAGENTA, body, BODY_YELLOW);
}

pub fn success(msg: &str) {
    tagged(glyph::ARROW_IN, TAG_GREEN, msg, BODY_GREEN);
}

pub fn error(msg: &str) {
    tagged(glyph::ARROW_IN, TAG_RED, msg, BODY_RED);
}

pub fn warning(msg: &str) {
    tagged(glyph::ARROW_IN, TAG_YELLOW, msg, BODY_YELLOW);
}

pub fn not_found(msg: &str) {
    warning(msg);
}

pub fn info(msg: &str) {
    line(&format!("{} {}", paint(glyph::ARROW_IN, TAG_CYAN), msg));
}

pub fn skip(msg: &str) {
    tagged(glyph::ARROW_IN, TAG_DIM, msg, BODY_DIM);
}

pub fn item(label: &str, value: &str) {
    let label = paint(&pad_display(label, LABEL_WIDTH), TAG_GREEN);
    let value = paint(value, BODY_YELLOW);
    line(&format!("{} {}", label, value));
}

pub fn detail(label: &str, value: &str) {
    let label = paint(&pad_display(label, LABEL_WIDTH + 2), TAG_DIM);
    line(&format!("{} {}", label, value));
}

pub fn message(msg: &str) {
    line(msg);
}

pub fn blank() {
    line("");
}

pub fn diff_old(s: &str) {
    line(&format!("{}{}", glyph::DIFF_OLD, paint(s, BODY_RED)));
}

pub fn diff_new(s: &str) {
    line(&format!("{}{}", glyph::DIFF_NEW, paint(s, BODY_GREEN)));
}

pub fn dry_run_header(msg: &str) {
    line("");
    line(&format!(
        "{} {}",
        paint(glyph::DRY_TAG, TAG_MAGENTA),
        paint(msg, TAG_CYAN),
    ));
}

// === Internal helpers ===

fn split_cmd(cmd: &str) -> (Option<&str>, &str) {
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

fn pad_display(s: &str, width: usize) -> String {
    let w = display_width(s);
    if w >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - w))
    }
}
