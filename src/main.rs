//! pma 可执行文件入口：注册各层模块，解析命令行，并在出错时打印完整错误链。
//!
//! 业务分层：`cli` 负责解析、`commands` 负责流程编排、`domain` 负责领域能力、
//! `model` 放纯数据结构、`engine` 负责把执行计划落地、`utils` 提供通用工具。

mod cli;
mod commands;
mod domain;
mod engine;
mod error;
mod model;
mod utils;

use crate::error::AppError;
use clap::Parser;
use std::error::Error;
use std::io::Write;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = cli::Cli::parse();
    match cli::dispatch(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            // 已经处在失败路径上，写 stderr 失败不应再引发 panic
            let mut stderr = std::io::stderr();
            render_error_chain(&err, &mut stderr);
            ExitCode::FAILURE
        }
    }
}

// 逐层展开 source()，让用户看到一条错误链而不是只剩最外层那句话
fn render_error_chain<W: Write>(err: &AppError, w: &mut W) {
    let _ = writeln!(w, "error: {}", err);
    let mut current: Option<&dyn Error> = err.source();
    while let Some(cause) = current {
        let _ = writeln!(w, "caused by: {}", cause);
        current = cause.source();
    }
}
