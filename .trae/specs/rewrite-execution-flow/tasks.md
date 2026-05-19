# Tasks

- [x] Task 1: 重写 `model/plan.rs` 数据结构
  - [x] SubTask 1.1: 定义 `DisplayMessage` 枚举（从 `MessageOperation` 迁移）
  - [x] SubTask 1.2: 定义 `Phase` 结构体（label + operations）
  - [x] SubTask 1.3: 定义 `PlanMetadata` 结构体（messages + dry_run）
  - [x] SubTask 1.4: 重写 `ExecutionPlan`（phases + metadata，封装字段，提供只读访问器）
  - [x] SubTask 1.5: 定义 `ExecutionResult` / `ExecutedOperation` / `SkippedOperation` / `OperationError` 结构体
  - [x] SubTask 1.6: 从 `Operation` 枚举中移除 `Message` 变体
  - [x] SubTask 1.7: 移除 `MessageOperation` 枚举及其 impl
  - [x] SubTask 1.8: 更新所有 `From<MessageOperation> for Operation` 等相关 impl

- [x] Task 2: 重写 `control/command.rs` — 统一 Command trait
  - [x] SubTask 2.1: 定义统一的 `Command` trait（Context + Plan 关联类型，collect/plan/execute 方法）
  - [x] SubTask 2.2: 移除 `MultiRepoCommand` trait
  - [x] SubTask 2.3: 为多仓库命令提供 `run_multi_repo()` 辅助函数

- [x] Task 3: 重写 `control/plan.rs` 执行引擎
  - [x] SubTask 3.1: 重写 `run_plan()` 支持阶段化执行（按 Phase 顺序执行）
  - [x] SubTask 3.2: 实现 `PlanMetadata.messages` 的渲染逻辑（替代原 MessageOperation 执行）
  - [x] SubTask 3.3: 重写 `execute_operation()` 返回 `ExecutionResult`
  - [x] SubTask 3.4: 重写 `display_plan()` 适配新的 ExecutionPlan 结构
  - [x] SubTask 3.5: 重写 `emit_recovery_hints()` 使用 `OperationError` 结构

- [x] Task 4: 适配 `commands/release.rs`
  - [x] SubTask 4.1: 将 `ReleaseGitState` 适配为新 Context 关联类型
  - [x] SubTask 4.2: 重写 `plan()` 使用 Phase 分组操作
  - [x] SubTask 4.3: 将 MessageOperation 替换为 DisplayMessage

- [x] Task 5: 适配 `commands/sync.rs`
  - [x] SubTask 5.1: 从 MultiRepoCommand 迁移到统一 Command trait
  - [x] SubTask 5.2: 重写 `plan()` 使用 Phase 分组操作
  - [x] SubTask 5.3: 将 MessageOperation 替换为 DisplayMessage

- [x] Task 6: 适配 `commands/status.rs`
  - [x] SubTask 6.1: 从 MultiRepoCommand 迁移到统一 Command trait
  - [x] SubTask 6.2: 重写 `plan()` 使用 Phase 分组操作
  - [x] SubTask 6.3: 将 MessageOperation 替换为 DisplayMessage

- [x] Task 7: 适配 `commands/branch.rs`
  - [x] SubTask 7.1: 从 MultiRepoCommand 迁移到统一 Command trait
  - [x] SubTask 7.2: 重写 `plan()` 使用 Phase 分组操作
  - [x] SubTask 7.3: 将 MessageOperation 替换为 DisplayMessage

- [x] Task 8: 适配 `commands/doctor.rs`
  - [x] SubTask 8.1: 从 MultiRepoCommand 迁移到统一 Command trait
  - [x] SubTask 8.2: 重写 `plan()` 使用 Phase 分组操作，不再直接访问 operations 字段
  - [x] SubTask 8.3: 将 MessageOperation 替换为 DisplayMessage

- [x] Task 9: 适配 `commands/fork.rs`
  - [x] SubTask 9.1: 重写 `plan()` 使用 Phase 分组操作
  - [x] SubTask 9.2: 将 MessageOperation 替换为 DisplayMessage

- [x] Task 10: 适配 `commands/gitlab.rs`
  - [x] SubTask 10.1: 重写 `plan()` 使用 Phase 分组操作
  - [x] SubTask 10.2: 将 MessageOperation 替换为 DisplayMessage

- [x] Task 11: 适配 `commands/config.rs`
  - [x] SubTask 11.1: 重写 `plan()` 使用 Phase 分组操作
  - [x] SubTask 11.2: 将 MessageOperation 替换为 DisplayMessage

- [x] Task 12: 适配 `commands/selfman.rs`
  - [x] SubTask 12.1: 重写 `plan()` 使用 Phase 分组操作
  - [x] SubTask 12.2: 将 MessageOperation 替换为 DisplayMessage

- [x] Task 13: 适配 `commands/snap.rs`
  - [x] SubTask 13.1: 重写 `plan()` 使用 Phase 分组操作
  - [x] SubTask 13.2: 将 MessageOperation 替换为 DisplayMessage

- [x] Task 14: 更新 `commands/mod.rs` 辅助函数
  - [x] SubTask 14.1: 更新 `run_multi_repo()` 适配新 Command trait
  - [x] SubTask 14.2: 更新 `init_repo_walker()` 签名（如需要）

- [x] Task 15: 编译验证与测试
  - [x] SubTask 15.1: `cargo check` 确保编译通过
  - [x] SubTask 15.2: `cargo test` 确保现有测试通过
  - [x] SubTask 15.3: 更新 `model/plan.rs` 中的测试用例

# Task Dependencies
- [Task 2] depends on [Task 1]
- [Task 3] depends on [Task 1]
- [Task 4..13] depend on [Task 1, Task 2, Task 3]
- [Task 14] depends on [Task 2]
- [Task 15] depends on [Task 4..14]
