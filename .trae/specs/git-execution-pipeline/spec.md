# Git 执行流程重构 Spec

## Why
当前各命令中 git 操作散落在各处，缺乏统一的执行流程。需要引入"上下文收集 → 执行计划 → 执行命令"三阶段模式，使 dry_run 时只展示计划不执行命令，同时让所有命令遵循一致的流程。

## What Changes
- 新增 `GitContext` 结构体，统一收集仓库上下文信息（root、branch、remotes、tags 等）
- 新增 `ExecutionPlan` 结构体，支持声明式操作编排，dry_run 时仅展示不执行
- 新增 `GitOperation` 枚举，定义所有可执行的 git 操作
- 重构所有使用 git 的命令使用三阶段流程：
  - `release.rs` - add/commit/tag/push
  - `sync.rs` - pull/push
  - `doctor.rs` - prune/set-upstream/remote rename/gc
  - `branch.rs` - branch delete/checkout/rename/push delete
  - `snap.rs` - init/add/commit/checkout
  - `fork.rs` - init/add/commit
  - `gitlab.rs` - clone
  - `status.rs` - 只读，使用 GitContext 替代分散查询
- 移除 `command.rs` 中已迁移到 `GitOperation` 的高级方法
- 移除 `remote.rs` 中的 `fix_remote_names`，逻辑合并到 `ExecutionPlan`
- 移除 `DryRunContext`（被 `ExecutionPlan` 替代）

## Impact
- Affected code: `src/domain/git/` (command.rs, models.rs, remote.rs, 新增 executor.rs)
- Affected code: `src/domain/runner/` (移除 DryRunContext)
- Affected code: `src/commands/` (release, sync, doctor, branch, snap, fork, gitlab, status)

## ADDED Requirements

### Requirement: GitContext 上下文收集
系统 SHALL 提供 `GitContext` 结构体，通过 `GitContext::collect(path)` 一次性收集仓库的所有上下文信息。

#### Scenario: 收集成功
- **WHEN** 调用 `GitContext::collect(repo_path)`
- **THEN** 返回包含 root、current_branch、remotes、branches、tags、has_uncommitted_changes 的上下文对象

#### Scenario: 非 git 仓库
- **WHEN** 路径不是 git 仓库
- **THEN** 返回 GitError

### Requirement: ExecutionPlan 执行计划
系统 SHALL 提供 `ExecutionPlan` 结构体，支持声明式添加操作，并在执行时根据 dry_run 标志决定是否真正执行。

#### Scenario: dry_run 模式
- **WHEN** `ExecutionPlan` 设置 `dry_run = true` 并调用 `execute()`
- **THEN** 仅展示将要执行的操作描述，不执行任何 git 命令

#### Scenario: 正常执行
- **WHEN** `ExecutionPlan` 设置 `dry_run = false` 并调用 `execute()`
- **THEN** 按顺序执行所有 `GitOperation`

### Requirement: GitOperation 操作枚举
系统 SHALL 提供 `GitOperation` 枚举，覆盖所有 git 写操作，每个变体可生成人类可读的描述。

#### Scenario: 操作描述
- **WHEN** 调用 `GitOperation::PushTag { remote: "origin", tag: "v1.0.0" }.description()`
- **THEN** 返回 `"git push origin v1.0.0"`

### Requirement: 三阶段流程统一
所有 git 命令 SHALL 遵循"上下文收集 → 执行计划 → 执行命令"三阶段流程。

#### Scenario: release 命令
- **WHEN** 执行 `pma release patch --dry-run`
- **THEN** 收集上下文 → 构建执行计划（add/commit/tag/push）→ dry_run 仅展示

#### Scenario: sync 命令
- **WHEN** 执行 `pma sync --dry-run`
- **THEN** 收集上下文 → 构建执行计划（pull/push）→ dry_run 仅展示

#### Scenario: doctor 命令
- **WHEN** 执行 `pma doctor -f --dry-run`
- **THEN** 收集上下文 → 诊断问题 → 构建修复计划 → dry_run 仅展示

#### Scenario: branch clean 命令
- **WHEN** 执行 `pma branch clean --dry-run`
- **THEN** 收集上下文 → 构建执行计划（branch -d/push --delete）→ dry_run 仅展示

#### Scenario: snap create 命令
- **WHEN** 执行 `pma snap create --dry-run`
- **THEN** 收集上下文 → 构建执行计划（add/commit）→ dry_run 仅展示

#### Scenario: fork 命令
- **WHEN** 执行 `pma fork --dry-run`
- **THEN** 收集上下文 → 构建执行计划（init/add/commit）→ dry_run 仅展示

#### Scenario: gitlab clone 命令
- **WHEN** 执行 `pma gitlab clone --dry-run`
- **THEN** 收集上下文 → 构建执行计划（clone）→ dry_run 仅展示

#### Scenario: status 命令（只读）
- **WHEN** 执行 `pma status`
- **THEN** 使用 `GitContext` 收集上下文 → 展示状态（无执行计划）

## MODIFIED Requirements

### Requirement: GitCommandRunner 职责收窄
`GitCommandRunner` SHALL 只保留底层命令执行和只读查询方法，写操作的高级封装移至 `GitOperation`。

### Requirement: remote.rs 精简
`remote.rs` SHALL 只保留 `resolve_remote_name` 和 `diagnose_remote_names` 诊断逻辑，修复逻辑通过 `ExecutionPlan` 实现。

### Requirement: 移除 DryRunContext
`DryRunContext` SHALL 被移除，其功能由 `ExecutionPlan` 的 dry_run 模式替代。
