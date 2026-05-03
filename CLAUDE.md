# CLAUDE.md

## 项目概述

`pma` (Project Manager Application) 是一个 Rust 命令行工具，用于批量管理 Git 仓库。
支持语义化版本发布、多仓库同步、仓库健康检查、模板项目 fork 和项目快照。

## 技术栈

- 语言: Rust (edition 2024)
- CLI 框架: clap 4.5 (derive 模式)
- 错误处理: anyhow + thiserror
- 序列化: serde / serde_json / toml / toml_edit
- 正则: regex
- 终端着色: colored, anstyle
- 进度条: indicatif
- HTTP: ureq
- XML: roxmltree

## 项目结构

```
src/
├── main.rs              # 入口: 解析 CLI → 分发命令
├── cli/                 # CLI 层: 参数定义、解析、路由
│   ├── mod.rs           # ParsedCommand / CommandArgs / trait 定义
│   ├── cli.rs           # clap derive 命令定义
│   ├── parser.rs        # clap → ParsedCommand 转换
│   └── dispatcher.rs    # ParsedCommand → Command::execute 路由
├── commands/            # 命令层: 各子命令的业务逻辑
│   ├── mod.rs           # Command trait + CommandError
│   ├── release.rs       # release 子命令
│   ├── sync.rs          # sync 子命令
│   ├── doctor.rs        # doctor 子命令
│   ├── fork.rs          # fork 子命令
│   ├── gitlab.rs        # gitlab 子命令
│   ├── snap.rs          # snap 子命令
│   ├── status.rs        # status 子命令
│   ├── branch.rs        # branch 子命令
│   ├── selfman.rs       # self update/version 子命令
│   └── config.rs        # config 子命令
├── domain/              # 领域层: 可复用的核心能力
│   ├── mod.rs           # DomainError 聚合
│   ├── git/             # Git 操作封装
│   │   ├── mod.rs       # GitError / GitProtocol / Result
│   │   ├── command.rs   # GitCommandRunner (执行 git 命令)
│   │   ├── remote.rs    # Remote / RemoteManager
│   │   └── repository.rs# RepoWalker / RepoInfo / Repository
│   ├── editor/          # 配置文件版本编辑器
│   │   ├── mod.rs       # FileEditor trait / EditorRegistry
│   │   ├── cargo_toml.rs
│   │   ├── package_json.rs
│   │   ├── pom_xml.rs
│   │   ├── pyproject.rs
│   │   ├── cmake.rs
│   │   ├── homebrew.rs
│   │   ├── project_py.rs
│   │   ├── version_text.rs
│   │   ├── file_types.rs
│   │   └── version_bump.rs
│   ├── config/          # 配置管理
│   │   ├── mod.rs       # ConfigError / ConfigManager trait
│   │   ├── schema.rs    # AppConfig 结构定义
│   │   └── manager.rs   # MultiSourceConfigManager
│   ├── gitlab/          # GitLab API 集成
│   │   ├── mod.rs       # GitLabError
│   │   ├── client.rs    # GitLabClient
│   │   ├── models.rs    # Project / Group / User
│   │   └── auth.rs      # AuthManager
│   └── runner/          # 命令执行抽象
│       ├── mod.rs       # RunnerError
│       └── dry_run.rs   # DryRunContext (共享的 dry-run 支持)
└── utils/               # 工具函数
    ├── mod.rs
    ├── path.rs          # 路径处理 (canonicalize_path, format_path)
    ├── git.rs           # 轻量 Git 工具函数
    └── file.rs          # (预留) 文件操作工具
```

## 架构分层

```
CLI 层 (cli/)
  ↓ ParsedCommand
命令层 (commands/)
  ↓ 调用
领域层 (domain/) + 工具层 (utils/)
```

- **CLI 层**: 只负责参数解析和路由，不含业务逻辑
- **命令层**: 实现具体业务流程，编排领域层能力
- **领域层**: 提供可复用的核心能力 (Git 操作、版本编辑、配置管理等)
- **工具层**: 通用工具函数 (路径处理、简单 Git 封装)

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
- 错误处理: 领域层用 `thiserror`，命令层用 `anyhow`
- 中文注释和错误消息

## 新增子命令步骤

1. 在 `src/cli/cli.rs` 的 `Commands` 枚举中添加变体
2. 在 `src/commands/` 下创建对应模块，实现 `Command` trait
3. 在 `src/commands/mod.rs` 中导出模块
4. 在 `src/cli/mod.rs` 的 `CommandName` 和 `CommandArgs` 中添加变体
5. 在 `src/cli/parser.rs` 中添加解析逻辑
6. 在 `src/cli/dispatcher.rs` 中添加路由

## 新增配置文件编辑器步骤

1. 在 `src/domain/editor/` 下创建新文件
2. 实现 `FileEditor` trait
3. 在 `editor/mod.rs` 中导出
4. 在 `EditorRegistry::default_with_editors()` 中注册

## 注意事项

- 二进制名称为 `pma`，不是 crate 名 `project-manager-app`
- release 命令只能在 master 分支执行
- sync 默认跳过 HTTPS 协议的 github.com、gitee.com 推送
- Windows 路径需处理 UNC 前缀 (`\\?\`)，参见 `utils::path`
- `DryRunContext` 是所有支持 `--dry-run` 的命令共享的，位于 `domain::runner::dry_run`
