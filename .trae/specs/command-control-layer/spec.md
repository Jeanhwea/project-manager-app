# 命令控制层与分层架构重构 Spec

## Why
当前 `executor.rs`（GitContext/ExecutionPlan/GitOperation）混在 domain/git 层中，但这些是命令控制逻辑而非领域逻辑。domain 层应只处理基本领域工作，命令编排应独立为控制层。中间交互数据需要定义 model 层，使三层职责清晰。

## What Changes
- 新增 `src/control/` 命令控制层，负责：收集上下文、制定执行计划、运行执行计划
- 新增 `src/model/` 模型层，定义控制层与 domain 层之间的交互数据结构
- 将 `GitContext`、`ExecutionPlan`、`GitOperation` 从 `domain/git/executor.rs` 移至 `control/`
- 将 `Remote`、`Branch`、`Tag` 从 `domain/git/models.rs` 移至 `model/`
- `domain/git/` 只保留 `GitCommandRunner`（命令执行）、`GitError`（错误）、`repository.rs`（仓库遍历）
- 移除 `domain/git/executor.rs`、`domain/git/models.rs`、`domain/git/remote.rs`
- `commands/` 中的各命令改为调用 `control/` 层

## Impact
- Affected code: `src/domain/git/` (精简为 command.rs + repository.rs + mod.rs + error)
- Affected code: `src/control/` (新增，从 executor.rs 迁移并扩展)
- Affected code: `src/model/` (新增，从 models.rs 迁移并扩展)
- Affected code: `src/commands/` (改为调用 control 层)
- Affected code: `src/domain/mod.rs` (移除 gitlab 模块引用调整)

## ADDED Requirements

### Requirement: model 层
系统 SHALL 提供 `src/model/` 模块层，定义控制层与 domain 层之间的交互数据结构。

#### Scenario: Git 数据模型
- **WHEN** 控制层需要 git 数据
- **THEN** 通过 `model::git::Remote`、`model::git::Branch`、`model::git::Tag` 等结构传递

#### Scenario: 执行操作模型
- **WHEN** 控制层构建执行计划
- **THEN** 通过 `model::plan::GitOperation`、`model::plan::ExecutionPlan` 描述操作

### Requirement: control 层
系统 SHALL 提供 `src/control/` 命令控制层，负责三阶段流程。

#### Scenario: 上下文收集
- **WHEN** 命令需要仓库信息
- **THEN** 通过 `control::GitContext::collect()` 收集，返回 `model::git::*` 数据

#### Scenario: 执行计划
- **WHEN** 命令需要执行 git 操作
- **THEN** 通过 `control::ExecutionPlan` 构建计划，dry_run 时仅展示不执行

#### Scenario: 运行执行计划
- **WHEN** `ExecutionPlan.execute()` 被调用
- **THEN** dry_run=true 时只打印计划，dry_run=false 时调用 domain 层执行

### Requirement: domain 层精简
domain/git 层 SHALL 只包含基本领域工作：命令执行和仓库遍历。

#### Scenario: GitCommandRunner
- **WHEN** 控制层需要执行 git 命令
- **THEN** 通过 `domain::git::GitCommandRunner` 执行，返回原始结果

## MODIFIED Requirements

### Requirement: commands 层调用 control
commands/ 中各命令 SHALL 通过 control 层完成工作，不再直接调用 domain 层。

### Requirement: remote 诊断逻辑
remote 诊断逻辑（resolve_remote_name/diagnose_remote_names）SHALL 移至 control 层。

## REMOVED Requirements

### Requirement: domain/git/executor.rs
**Reason**: 控制逻辑不属于 domain 层，移至 control/
**Migration**: 移至 `src/control/context.rs` 和 `src/control/plan.rs`

### Requirement: domain/git/models.rs
**Reason**: 数据模型独立为 model 层
**Migration**: 移至 `src/model/git.rs`

### Requirement: domain/git/remote.rs
**Reason**: 诊断逻辑属于控制层
**Migration**: 移至 `src/control/remote.rs`
