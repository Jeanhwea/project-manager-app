# PMA 架构文档

## 概述

PMA (Project Manager Application) 是一个 Rust CLI 工具，用于管理多个代码仓库的版本发布、同步、诊断等操作。

## 架构分层

```
┌─────────────────────────────────────────────┐
│                  CLI 层                      │  src/cli/
│  命令行参数定义 (clap derive)                │
├─────────────────────────────────────────────┤
│               Commands 层                    │  src/commands/
│  业务编排: context → plan → execute          │
├─────────────────────────────────────────────┤
│               Control 层                     │  src/control/
│  执行引擎: plan.rs (运行 ExecutionPlan)      │
│  命令 trait: command.rs (Command/MultiRepo)  │
├─────────────────────────────────────────────┤
│               Domain 层                      │  src/domain/
│  核心领域逻辑:                               │
│  - git/       Git 操作与上下文               │
│  - editor/    文件编辑与版本修改              │
│  - config/    配置管理                       │
│  - runner/    命令执行器                     │
│  - selfupdate/ 自更新                        │
├─────────────────────────────────────────────┤
│             Model/Utils 层                   │  src/model/ + src/utils/
│  数据模型与工具函数                           │
└─────────────────────────────────────────────┘
```

## 核心设计模式

### 三阶段执行模式

所有命令遵循 `context() → plan() → execute()` 三阶段分离:

1. **context()** - 收集执行所需的上下文信息（Git 状态、远程仓库等）
2. **plan()** - 构建 `ExecutionPlan`，包含要执行的操作列表
3. **execute()** - 由 `control::plan::run_plan` 统一执行

这种分离带来以下优势:
- **dry-run 支持**: plan 阶段不执行副作用，可安全预览
- **可测试性**: 可以独立测试 plan 的输出
- **可恢复性**: 执行失败时提供恢复指引

### Command / MultiRepoCommand Trait

```rust
trait Command {
    type Context;
    fn context(&self) -> Result<Self::Context>;
    fn plan(&self, ctx: &Self::Context) -> Result<ExecutionPlan>;
}

trait MultiRepoCommand {
    type Context;
    fn context(&self, repo_path: &Path) -> Result<Self::Context>;
    fn plan(&self, ctx: &Self::Context) -> Result<ExecutionPlan>;
}
```

- `Command`: 单仓库命令（如 release, fork, self update）
- `MultiRepoCommand`: 多仓库命令（如 sync, status, branch, doctor）

### ExecutionPlan 统一操作模型

```rust
enum Operation {
    Git(GitOperation),        // Git 命令
    Shell(ShellOperation),    // Shell 命令
    Edit(EditOperation),      // 文件编辑
    SelfUpdate(SelfUpdateOperation), // 自更新
    Message(MessageOperation), // 输出消息
}
```

所有副作用操作统一建模为 Operation，由 `control::plan::run_plan` 统一执行。

### FileEditor 注册表模式

```rust
trait FileEditor {
    fn name(&self) -> &str;
    fn file_patterns(&self) -> &[&str];
    fn candidate_files(&self) -> Vec<&str>;
    fn find_version(&self, content: &str) -> Option<VersionPosition>;
}
```

通过 `EditorRegistry` 动态注册编辑器，支持:
- 自动检测配置文件类型
- 动态候选文件列表
- 版本号定位与修改

## 模块说明

### src/domain/git/

Git 操作的核心领域模块:

| 文件 | 职责 |
|------|------|
| command.rs | GitCommandRunner: Git 命令执行封装 |
| context.rs | collect_context / collect_context_with_runner: Git 上下文收集 |
| release.rs | 版本发布相关: 状态验证、Git 根目录解析 |
| diagnose.rs | 仓库诊断: 检测 detached head、陈旧引用等 |
| remote.rs | 远程仓库操作: 名称诊断、主机提取 |
| repository.rs | RepoWalker: Git 仓库发现与遍历 |

### src/domain/editor/

文件编辑模块:

| 文件 | 职责 |
|------|------|
| mod.rs | FileEditor trait、EditorRegistry、各编辑器实现 |
| detect.rs | 配置文件检测、glob 展开、lockfile 操作 |
| version_bump.rs | Version 类型（基于 semver）、版本号递增 |

### src/domain/config/

配置管理模块:

| 文件 | 职责 |
|------|------|
| schema.rs | 配置数据结构定义 (AppConfig, GitLabConfig) |
| manager.rs | ConfigManager: 配置加载与缓存 (OnceLock) |

### src/domain/runner/

命令执行器:

| 文件 | 职责 |
|------|------|
| command.rs | CommandRunner: 跨平台命令执行封装 |

### src/domain/selfupdate/

自更新模块:

| 文件 | 职责 |
|------|------|
| updater.rs | GitHub Release API、资源下载、二进制安装 |

## 配置文件

### 应用配置 (~/.config/pma/config.toml)

```toml
[repository]
skip_dirs = ["node_modules", ".venv", "target", "__pycache__"]

[sync]
skip_push_remotes = ["internal"]
```

### GitLab 配置 (~/.config/pma/gitlab.toml)

```toml
[gitlab]
url = "https://gitlab.example.com"
token = "glpat-xxx"
```

## 错误处理

使用 `thiserror` 定义领域错误类型:

- `AppError`: 顶层应用错误（包含 Release、SelfUpdate、Io 等变体）
- `GitError`: Git 操作错误
- `EditorError`: 文件编辑错误

所有错误通过 `Result<T>` 向上传播，CLI 层统一处理。
