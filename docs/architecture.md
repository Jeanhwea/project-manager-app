# PMA 架构文档

## 概述

PMA (Project Manager Application) 是一个 Rust CLI 工具，用于管理多个代码仓库的版本发布、同步、诊断等操作。

**技术栈**: Rust 2024 Edition | clap (derive) | thiserror | serde | semver | ureq

## 架构分层

```
┌──────────────────────────────────────────────────────────────────┐
│                    CLI 层 (src/cli/)                            │
│  命令行参数定义 (clap derive) + 样式配置 + 分发逻辑              │
├──────────────────────────────────────────────────────────────────┤
│                 Commands 层 (src/commands/)                     │
│  业务编排: context() → plan() → execute()                       │
│  11 个命令模块: release, sync, status, branch, doctor, fork,   │
│                gitlab, config, selfman, snap                    │
├──────────────────────────────────────────────────────────────────┤
│                 Control 层 (src/control/)                       │
│  command.rs:  Command / MultiRepoCommand trait                  │
│  plan.rs:     ExecutionPlan 执行引擎 (dry-run / 恢复指引)       │
├──────────────────────────────────────────────────────────────────┤
│                 Domain 层 (src/domain/)                         │
│  git/       Git 操作与上下文收集 (21 种 GitOperation)           │
│  editor/    文件编辑注册表 (7 种编辑器 + Version 类型)          │
│  config/    配置管理 (AppConfig / GitLabConfig)                 │
│  runner/    跨平台命令执行器 (Capture / Streaming)              │
│  selfupdate/ GitHub Release API 自更新                         │
├──────────────────────────────────────────────────────────────────┤
│              Model/Utils 层                                     │
│  model/git.rs:   GitContext, Remote, Branch, Tag 数据模型       │
│  model/plan.rs:  Operation 枚举 (Git/Shell/Edit/SelfUpdate/Msg) │
│  utils/output.rs: 彩色终端输出 (带后端抽象)                      │
│  utils/path.rs:   路径规范化 (Windows \\?\ 处理)               │
└──────────────────────────────────────────────────────────────────┘
```

## 命令矩阵

| 命令 | 类型 | 核心职责 | 上下文 |
|------|------|----------|--------|
| `release` | Command | 版本发布 (bump + commit + tag + push) | ReleaseGitState |
| `sync` | MultiRepoCommand | 多仓库同步 (pull/push 所有分支) | SyncContext |
| `status` | MultiRepoCommand | 多仓库状态展示 | StatusContext |
| `branch` | MultiRepoCommand | 分支管理 (list/clean/switch/rename/all) | BranchContext |
| `doctor` | MultiRepoCommand | 仓库健康诊断 + 自动修复 | DoctorContext |
| `fork` | Command | 项目模板复制 | ForkContext |
| `gitlab login` | Command | GitLab 凭证配置 | LoginContext |
| `gitlab clone` | Command | GitLab 批量克隆 | CloneProject |
| `config init/show/path` | Command | 配置文件管理 | ConfigInitContext |
| `selfmanage update/version` | Command | 自更新 + 版本显示 | SelfUpdateContext |
| `snap create/list/restore` | Command | 项目快照管理 | SnapContext |

## 核心设计模式

### 三阶段执行模式

所有命令遵循 `context() → plan() → execute()` 三阶段分离:

1. **context()** - 收集执行所需的上下文信息（Git 状态、远程仓库等）
2. **plan()** - 构建 `ExecutionPlan`，包含要执行的操作列表
3. **execute()** - 由 `control::plan::run_plan` 统一执行

```
┌──────────┐     ┌──────────┐     ┌──────────┐
│ Context  │────▶│  Plan    │────▶│ Execute  │
│  收集数据 │     │ 构建操作  │     │ 统一执行  │
└──────────┘     └──────────┘     └──────────┘
   Command::        Execution        run_plan()
   context()        Plan::new()       + dry-run
```

这种分离带来以下优势:
- **dry-run 支持**: plan 阶段不执行副作用，可安全预览
- **可测试性**: 可以独立测试 plan 的输出
- **可恢复性**: 执行失败时提供恢复指引（已执行操作 + 手动补救命令）

### Command / MultiRepoCommand Trait

```rust
// 单仓库命令 (release, fork, gitlab, config, selfmanage, snap)
trait Command {
    type Context;
    fn context(&self) -> Result<Self::Context>;
    fn plan(&self, ctx: &Self::Context) -> Result<ExecutionPlan>;
    fn run(&self) -> Result<()>;  // context → plan → execute
}

// 多仓库命令 (sync, status, branch, doctor)
trait MultiRepoCommand {
    type Context;
    fn context(&self, repo_path: &Path) -> Result<Self::Context>;
    fn plan(&self, ctx: &Self::Context, repo_path: &Path) -> Result<ExecutionPlan>;
    fn run(&self, walker: &RepoWalker) -> Result<()>;  // 遍历 + 逐个执行
}
```

- `Command`: 单仓库命令（如 release, fork, self update）
- `MultiRepoCommand`: 多仓库命令（如 sync, status, branch, doctor），内部使用 `RepoWalker` 遍历

### ExecutionPlan 统一操作模型

```rust
pub enum Operation {
    Git(GitOperation),           // 21 种 Git 操作
    Shell(ShellOperation),       // Shell 命令
    Edit(EditOperation),         // 文件编辑 (WriteFile / CopyDir)
    SelfUpdate(SelfUpdateOperation),
    Message(MessageOperation),   // 输出消息 (不产生副作用)
}

pub enum GitOperation {
    Init { working_dir },
    Clone { url, target_dir, working_dir },
    Add { path, working_dir },
    Commit { message, working_dir },
    CreateTag { tag, working_dir },
    PushTag { remote, tag, working_dir },
    PushBranch { remote, branch, working_dir },
    PushAll { remote, working_dir },
    PushTags { remote, working_dir },
    Pull { remote, branch, working_dir },
    Checkout { ref_name, working_dir },
    DeleteBranch { branch, working_dir },
    RenameBranch { old, new, working_dir },
    DeleteRemoteBranch { remote, branch, working_dir },
    RenameRemote { old, new, working_dir },
    PruneRemote { remote, working_dir },
    SetUpstream { remote, branch, working_dir },
    Gc { working_dir },
}
```

所有副作用操作统一建模为 Operation，由 `control::plan::run_plan` 统一执行。

### FileEditor 注册表模式

```rust
pub trait FileEditor: Send + Sync {
    fn name(&self) -> &str;
    fn file_patterns(&self) -> &[&str];
    fn candidate_files(&self) -> Vec<&str>;
    fn find_version(&self, content: &str) -> Option<VersionPosition>;
    fn parse(&self, content: &str) -> Result<VersionLocation>;
    fn edit(&self, content: &str, location: &VersionLocation, new_version: &str) -> Result<String>;
    fn validate(&self, original: &str, edited: &str) -> Result<()>;
}
```

通过 `EditorRegistry` 动态注册编辑器，支持:
- 自动检测配置文件类型
- 动态候选文件列表（支持 `{parent}` / `{}` 占位符）
- 版本号定位与修改

**已实现的编辑器**:

| 编辑器 | 文件格式 | 文件模式 |
|--------|----------|----------|
| CargoTomlEditor | Cargo.toml | `Cargo.toml` |
| CMakeListsEditor | CMakeLists.txt | `CMakeLists.txt` |
| HomebrewFormulaEditor | Homebrew Formula | `{parent}.rb` |
| PackageJsonEditor | package.json | `package.json` |
| PomXmlEditor | pom.xml | `pom.xml` |
| PythonVersionEditor | project.py | `project.py` |
| PyprojectEditor | pyproject.toml | `pyproject.toml` |
| TauriConfEditor | tauri.conf.json | `tauri.conf.json` |
| VersionTextEditor | 纯文本版本文件 | 自定义 |

## 模块说明

### src/cli/

| 文件 | 职责 |
|------|------|
| `mod.rs` | 终端样式配置 (anstyle) |
| `commands.rs` | `Cli` 结构体 + `Commands` 枚举 + `dispatch()` 分发逻辑 |

### src/commands/

| 文件 | 职责 |
|------|------|
| `mod.rs` | `RepoPathArgs` + `init_repo_walker()` + `run_multi_repo()` |
| `release.rs` | 版本发布: bump 策略 (major/minor/patch)、pre-release、force 模式 |
| `sync.rs` | 多仓库同步: pull/push 所有分支、remote 过滤 |
| `status.rs` | 多仓库状态: 分支、未提交变更、远程列表 |
| `branch.rs` | 分支管理: list/clean/switch/rename/all (5 个子命令) |
| `doctor.rs` | 仓库诊断: detached head、陈旧引用、stash、仓库大小等 |
| `fork.rs` | 项目模板复制: 目录拷贝 + 清理 .git |
| `gitlab.rs` | GitLab 集成: login (凭证配置) + clone (批量克隆) |
| `config.rs` | 配置管理: init/show/path 子命令 |
| `selfman.rs` | 自管理: update (GitHub Release) + version 显示 |
| `snap.rs` | 快照管理: create/list/restore (基于 Git 快照) |

### src/control/

| 文件 | 职责 |
|------|------|
| `command.rs` | `Command` / `MultiRepoCommand` trait 定义 + 默认 `run()` 实现 |
| `plan.rs` | `run_plan()` 执行引擎: dry-run 预览、错误恢复指引、working_dir 解析 |

### src/domain/git/

Git 操作的核心领域模块:

| 文件 | 职责 |
|------|------|
| `command.rs` | `GitCommandRunner`: Git 命令执行封装 (21 种操作) |
| `context.rs` | `collect_context()` / `collect_context_with_runner()`: Git 上下文收集 |
| `release.rs` | 版本发布相关: `resolve_git_root()` / `validate_git_state()` |
| `diagnose.rs` | 仓库诊断: `Diagnosis` 枚举 + `diagnose_repo()` |
| `remote.rs` | 远程仓库操作: `resolve_remote_name()` / `diagnose_remote_names()` |
| `repository.rs` | `RepoWalker`: Git 仓库发现与递归遍历 (支持 skip_dirs) |

### src/domain/editor/

文件编辑模块:

| 文件 | 职责 |
|------|------|
| `mod.rs` | `FileEditor` trait、`EditorRegistry`、`matches_file()` 模式匹配 |
| `detect.rs` | 配置文件检测、glob 展开 (`{}` / `{parent}` 占位符)、lockfile 操作 |
| `version_bump.rs` | `Version` 类型 (基于 semver)、`BumpType` 枚举 |
| `cargo_toml.rs` | Cargo.toml 编辑器 (workspace / package 双模式) |
| `cmake.rs` | CMakeLists.txt 编辑器 |
| `homebrew.rs` | Homebrew Formula 编辑器 |
| `package_json.rs` | package.json 编辑器 |
| `pom_xml.rs` | pom.xml 编辑器 (roxmltree) |
| `project_py.rs` | Python project.py 编辑器 |
| `pyproject.rs` | pyproject.toml 编辑器 |
| `tauri_conf.rs` | tauri.conf.json 编辑器 (Tauri 应用配置) |
| `version_text.rs` | 纯文本版本文件编辑器 |

### src/domain/config/

配置管理模块:

| 文件 | 职责 |
|------|------|
| `schema.rs` | `AppConfig` / `RepositoryConfig` / `RemoteConfig` / `GitLabConfig` 数据结构 |
| `manager.rs` | `ConfigManager`: 配置加载与缓存 (OnceLock)，路径 `~/.pma/` |

### src/domain/runner/

命令执行器:

| 文件 | 职责 |
|------|------|
| `command.rs` | `CommandRunner`: 跨平台命令执行 (Capture / Streaming 两种模式) |
| `mod.rs` | `ExecutionContext` builder、`CommandResult`、`OutputMode` 枚举 |

### src/domain/selfupdate/

自更新模块:

| 文件 | 职责 |
|------|------|
| `updater.rs` | GitHub Release API、6 个代理镜像回退、二进制下载与安装 |

### src/model/

| 文件 | 职责 |
|------|------|
| `git.rs` | `GitContext` / `Remote` / `Branch` / `Tag` 数据模型 |
| `plan.rs` | `Operation` / `GitOperation` / `ShellOperation` / `EditOperation` / `ExecutionPlan` |

### src/utils/

| 文件 | 职责 |
|------|------|
| `output.rs` | 彩色终端输出 (`colored`)，带 `OutputBackend` trait 抽象 |
| `path.rs` | 路径规范化 (Windows `\\?\` 前缀处理) |

## 错误处理

使用 `thiserror` 定义领域错误类型:

| 错误类型 | 来源 | 覆盖范围 |
|----------|------|----------|
| `AppError` | 顶层应用错误 | 所有子错误 + 领域特定错误 |
| `GitError` | `domain::git` | Git 命令失败 / IO |
| `EditorError` | `domain::editor` | 解析 / 写入 / 版本格式 / 格式保留 |

**AppError 变体**: CommandNotAvailable, Editor, Git, Io, NotFound, AlreadyExists, NotSupported, InvalidInput, GitLabApi, Release, SelfUpdate, Snapshot, Regex, ParseInt, SemVer

所有错误通过 `Result<T>` 向上传播，CLI 层统一处理。

## 配置文件

### 应用配置 (~/.pma/config.toml)

```toml
[repository]
max_depth = 3
skip_dirs = ["node_modules", ".venv", "target", "__pycache__", ...]

[remote]
# remote 名称映射规则

[sync]
skip_push_remotes = ["internal"]
```

### GitLab 配置 (~/.pma/gitlab.toml)

```toml
[gitlab]
url = "https://gitlab.example.com"
token = "glpat-xxx"
```

### 项目级配置 (`<repo>/.pma.json`)

仅用于 `pma release`：当未通过 CLI 指定文件时，会读取此文件中的 `files` 列表作为待升级版本的目标。优先级：CLI 参数 > `.pma.json` > 自动探测。

```json
{
  "files": [
    "Cargo.toml",
    "src-tauri/tauri.conf.json",
    "npm/pma/package.json"
  ]
}
```

## 数据流图

```
                    ┌──────────┐
                    │  main()  │
                    │  clap::  │
                    │  Parser  │
                    └────┬─────┘
                         │
                    ┌────▼─────┐
                    │  Cli     │
                    │ dispatch │
                    └────┬─────┘
                         │
              ┌──────────┼──────────┐
              ▼          ▼          ▼
         ┌────────┐ ┌────────┐ ┌────────┐
         │Command │ │Command │ │Multi   │
         │(单仓库)│ │(单仓库)│ │Repo    │
         └───┬────┘ └───┬────┘ │(多仓库)│
             │          │      └───┬────┘
             │          │          │
         ┌───▼────┐ ┌──▼─────┐ ┌──▼─────┐
         │ context│ │  plan  │ │ walker │
         │  收集  │ │ 构建   │ │ 遍历   │
         └────┬───┘ └──┬─────┘ └──┬─────┘
              │       │          │
              └───────┼──────────┘
                      ▼
               ┌─────────────┐
               │ Execution   │
               │   Plan      │
               │  (Operations)│
               └──────┬──────┘
                      │
               ┌──────▼──────┐
               │  run_plan() │
               │  统一执行    │
               └──────┬──────┘
                      │
            ┌─────────┼─────────┐
            ▼         ▼         ▼
         ┌──────┐ ┌──────┐ ┌──────┐
         │ Git  │ │ Shell│ │ Edit │
         │ Cmd  │ │ Cmd  │ │ File │
         └──────┘ └──────┘ └──────┘
```

## 依赖关系

```
pma (binary)
├── clap 4.5        # CLI 参数解析
├── serde/serde_json # 序列化
├── toml/toml_edit   # TOML 配置
├── semver 1.0       # 语义化版本
├── thiserror 1.0    # 错误类型
├── ureq 2.12        # HTTP 客户端 (自更新)
├── colored 3.1      # 终端颜色
├── anstyle 1.0      # CLI 样式
├── roxmltree 0.20   # XML 解析 (pom.xml)
├── dirs 5.0         # 用户目录
├── regex 1          # 模式匹配
├── indicatif 0.18   # 进度条
├── url 2.5          # URL 解析
├── anyhow 1.0       # 通用错误
├── flate2/tar/zip   # 压缩/解压 (自更新)
└── rpassword 7      # 密码输入
```
