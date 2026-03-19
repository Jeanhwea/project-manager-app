# CLAUDE.md

## 项目概述

`pma` (Project Manager Application) 是一个 Rust 命令行工具，用于批量管理 Git 仓库。
支持语义化版本发布、多仓库同步、仓库健康检查、模板项目 fork 和项目快照。

## 技术栈

- 语言: Rust (edition 2024)
- CLI 框架: clap 4.5 (derive 模式)
- 错误处理: anyhow
- 序列化: serde / serde_json
- 正则: regex
- 终端着色: colored, anstyle

## 项目结构

```
src/
├── main.rs          # 入口，CLI 分发
├── cli.rs           # clap 命令定义 (Parser + Subcommand)
├── utils.rs         # 工具函数 (路径处理等)
└── app/
    ├── mod.rs       # 模块导出
    ├── runner.rs    # 外部命令执行封装
    ├── git.rs       # Git 操作封装
    ├── version.rs   # 语义化版本解析与比较
    ├── repo.rs      # Git 仓库发现与遍历
    ├── release.rs   # release 子命令
    ├── sync.rs      # sync 子命令
    ├── doctor.rs    # doctor 子命令
    ├── fork.rs      # fork 子命令
    └── snap.rs      # snap 子命令
```

## 常用命令

```bash
# 编译
cargo build
cargo build --release

# 运行
cargo run -- <subcommand>

# 测试
cargo test

# 格式化
./fmt.cmd
# 或
cargo fmt
```

## 代码风格

- 使用 rustfmt 格式化，配置见 `rustfmt.toml`
- 最大行宽: 98
- edition: 2024
- `merge_derives = false`，每个 derive 宏独立书写
- 错误处理统一使用 `anyhow::Result`，配合 `.context()` 提供上下文信息
- 中文注释和错误消息
- 每个子命令在 `src/app/` 下独立文件，暴露 `pub fn execute(...)` 作为入口
- CLI 定义集中在 `src/cli.rs`，使用 clap derive 宏

## 架构约定

- `src/app/runner.rs` 封装所有外部命令调用，不要在其他模块直接调用 `std::process::Command`
- `src/app/git.rs` 封装 Git 操作，基于 runner 模块
- `src/app/repo.rs` 负责仓库发现和遍历逻辑
- 新增子命令时:
  1. 在 `src/cli.rs` 的 `Commands` 枚举中添加变体
  2. 在 `src/app/` 下创建对应模块文件，实现 `pub fn execute(...) -> Result<()>`
  3. 在 `src/app/mod.rs` 中导出模块
  4. 在 `src/main.rs` 的 match 中添加分发逻辑

## 注意事项

- 二进制名称为 `pma`，不是 crate 名 `project-manager-app`
- release 命令只能在 master 分支执行
- sync 默认跳过 HTTPS 协议的 github.com、gitee.com 推送
- Windows 路径需处理 UNC 前缀 (`\\?\`)，参见 `utils::format_path`
