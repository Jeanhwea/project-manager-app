//! 命令行层：定义 pma 的顶层 CLI 结构、子命令分发，以及统一的 clap 输出配色。
//! 子命令的参数结构和 `run` 实现放在 `commands` 模块，这里只做转发。

mod commands;

pub use commands::{Cli, dispatch};

use clap::builder::styling::Styles;

/// clap 帮助/报错信息的统一样式：标题黄、字面量绿、错误红、候选项青
pub fn styles() -> Styles {
    Styles::styled()
        .header(
            anstyle::Style::new()
                .bold()
                .underline()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow))),
        )
        .literal(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green))),
        )
        .placeholder(
            anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::White))),
        )
        .error(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red))),
        )
        .valid(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Cyan))),
        )
        .invalid(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red))),
        )
}
