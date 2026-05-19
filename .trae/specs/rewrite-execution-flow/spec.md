# 执行流程重写 Spec

## Why
当前执行流程存在架构缺陷：`ExecutionPlan` 仅是 `Vec<Operation>` 的扁平列表，缺乏结构化阶段划分；`MessageOperation` 与副作用操作混杂在同一列表中，导致输出与执行耦合；`Command` / `MultiRepoCommand` trait 的 Context 类型参数化不统一；`ExecutionPlan.operations` 直接暴露为 `pub`；Doctor 命令绕过标准框架直接访问内部字段。需要重写执行流程，引入结构化数据传递，使 context 收集、plan 构建、command 执行三阶段通过明确的数据结构衔接。

## What Changes
- **BREAKING**: 重写 `ExecutionPlan`，引入阶段化结构（`Phase`），将操作按阶段分组（Collect / Plan / Execute），每个阶段有明确的输入输出数据结构
- **BREAKING**: 重写 `Command` / `MultiRepoCommand` trait，统一为单一的 `Command` trait，通过泛型关联类型约束 Context 和 Plan 的数据流
- **BREAKING**: 移除 `MessageOperation`，将展示信息从操作列表中分离，改为 `ExecutionPlan` 的元数据
- 重写 `control/plan.rs` 执行引擎，支持阶段化执行和结构化结果传递
- 重写所有 11 个命令模块，适配新的执行框架
- 封装 `ExecutionPlan` 内部字段，提供只读访问器

## Impact
- Affected specs: 所有命令的执行流程
- Affected code:
  - `src/model/plan.rs` — Operation 枚举和 ExecutionPlan 重写
  - `src/control/command.rs` — Command trait 重写
  - `src/control/plan.rs` — 执行引擎重写
  - `src/commands/*.rs` — 所有 11 个命令模块适配
  - `src/model/git.rs` — Context 数据结构可能调整

## ADDED Requirements

### Requirement: 结构化执行阶段
系统 SHALL 将执行流程划分为三个明确阶段，每个阶段通过独立的数据结构传递信息：

1. **Collect 阶段**：收集上下文信息，输出 `CommandContext`
2. **Plan 阶段**：基于上下文构建执行计划，输出 `ExecutionPlan`
3. **Execute 阶段**：执行计划中的操作，输出 `ExecutionResult`

#### Scenario: 正常三阶段执行
- **WHEN** 用户执行任意命令
- **THEN** 系统依次执行 Collect → Plan → Execute 三个阶段
- **AND** 每个阶段的输出作为下一阶段的输入
- **AND** 任意阶段失败时立即中止并返回错误

#### Scenario: dry-run 模式
- **WHEN** 用户指定 dry-run 标志
- **THEN** 系统执行 Collect 和 Plan 阶段，跳过 Execute 阶段
- **AND** 显示计划内容但不执行任何副作用操作

### Requirement: 统一 Command Trait
系统 SHALL 提供统一的 `Command` trait，同时支持单仓库和多仓库命令：

```rust
trait Command {
    type Context;
    type Plan;

    fn collect(&self) -> Result<Self::Context>;
    fn plan(&self, ctx: &Self::Context) -> Result<Self::Plan>;
    fn execute(&self, plan: &Self::Plan) -> Result<ExecutionResult>;
}
```

#### Scenario: 单仓库命令实现
- **WHEN** 命令只操作单个仓库（如 release, fork）
- **THEN** `collect()` 自行确定工作目录并收集上下文
- **AND** `plan()` 基于上下文构建计划
- **AND** `execute()` 执行计划

#### Scenario: 多仓库命令实现
- **WHEN** 命令操作多个仓库（如 sync, status, branch, doctor）
- **THEN** `collect()` 接收仓库路径并收集上下文
- **AND** 命令自行管理仓库遍历逻辑
- **AND** 每个仓库独立执行 collect → plan → execute 流程

### Requirement: 结构化 ExecutionPlan
系统 SHALL 将 `ExecutionPlan` 重构为包含阶段化操作的结构：

```rust
struct ExecutionPlan {
    phases: Vec<Phase>,
    metadata: PlanMetadata,
}

struct Phase {
    label: String,
    operations: Vec<Operation>,
}

struct PlanMetadata {
    messages: Vec<DisplayMessage>,  // 从 Operation 中分离的展示信息
    dry_run: bool,
}
```

#### Scenario: 计划包含多个阶段
- **WHEN** release 命令构建计划
- **THEN** 计划包含 "版本修改" 阶段（Edit + Add 操作）和 "Git 提交推送" 阶段（Commit + Tag + Push 操作）
- **AND** 每个阶段有明确的标签描述

#### Scenario: 展示信息与副作用操作分离
- **WHEN** 命令需要输出信息（如 Header, Section, Item）
- **THEN** 这些信息存储在 `PlanMetadata.messages` 中
- **AND** 不与 Git/Shell/Edit 操作混杂在同一列表中

### Requirement: ExecutionResult 执行结果
系统 SHALL 在执行完成后返回结构化结果：

```rust
struct ExecutionResult {
    executed: Vec<ExecutedOperation>,
    skipped: Vec<SkippedOperation>,
    errors: Vec<OperationError>,
}

struct ExecutedOperation {
    description: String,
    phase: String,
}

struct SkippedOperation {
    description: String,
    reason: String,
}

struct OperationError {
    description: String,
    phase: String,
    recovery_hint: Option<String>,
}
```

#### Scenario: 部分操作失败
- **WHEN** 执行过程中某个操作失败
- **THEN** 系统返回 `ExecutionResult`，包含已执行操作列表和失败操作信息
- **AND** 失败操作包含恢复提示

#### Scenario: 所有操作成功
- **WHEN** 所有操作执行成功
- **THEN** 系统返回 `ExecutionResult`，`errors` 为空
- **AND** `executed` 包含所有已执行操作的描述

### Requirement: 封装 ExecutionPlan 内部字段
系统 SHALL 封装 `ExecutionPlan` 的内部字段，仅通过方法访问：

#### Scenario: 外部模块访问操作列表
- **WHEN** 外部模块需要访问计划中的操作
- **THEN** 通过 `plan.operations()` / `plan.phases()` 等只读方法访问
- **AND** 不能直接修改 `operations` 字段

### Requirement: 移除 MessageOperation
系统 SHALL 从 `Operation` 枚举中移除 `MessageOperation` 变体，将展示信息改为 `DisplayMessage` 结构体存储在 `PlanMetadata` 中。

#### Scenario: 命令需要输出信息
- **WHEN** 命令需要展示标题、节标题、键值对等信息
- **THEN** 使用 `DisplayMessage` 枚举添加到 `PlanMetadata.messages`
- **AND** 执行引擎在对应阶段前后渲染这些消息

## MODIFIED Requirements

### Requirement: Command Trait 统一
原 `Command` 和 `MultiRepoCommand` 两个 trait 合并为统一的 `Command` trait。多仓库命令通过在 `collect()` 中接收仓库路径参数（使用 `MultiRepoCommand` 标记 trait 或泛型约束区分），不再需要独立的 `MultiRepoCommand` trait。

### Requirement: Operation 枚举简化
`Operation` 枚举移除 `Message` 变体，仅保留有副作用的操作类型：
- `Git(GitOperation)`
- `Shell(ShellOperation)`
- `Edit(EditOperation)`
- `SelfUpdate(SelfUpdateOperation)`

## REMOVED Requirements

### Requirement: MultiRepoCommand Trait
**Reason**: 与 `Command` trait 合并，统一执行框架。多仓库命令通过 `Command` trait + 自行管理遍历逻辑实现。
**Migration**: 原 `MultiRepoCommand` 实现者改为实现 `Command` trait，在 `collect()` 中接受仓库路径。

### Requirement: MessageOperation 作为 Operation 变体
**Reason**: 展示信息不应与副作用操作混杂，分离后可独立控制输出时机和格式。
**Migration**: 所有 `MessageOperation` 用法改为 `DisplayMessage`，存储在 `PlanMetadata.messages` 中。
