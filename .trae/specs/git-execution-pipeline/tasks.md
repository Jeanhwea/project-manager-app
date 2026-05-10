# Tasks

- [x] Task 1: 实现 GitContext 和 ExecutionPlan
  - [x] 1.1: 创建 `src/domain/git/executor.rs`，定义 `GitContext`
  - [x] 1.2: 定义 `GitOperation` 枚举
  - [x] 1.3: 定义 `ExecutionPlan`
  - [x] 1.4: 在 `mod.rs` 中注册 `executor` 模块
  - [x] 1.5: 编译验证

- [x] Task 2: 精简 GitCommandRunner
  - [x] 2.1: 移除高级写方法
  - [x] 2.2: 保留底层执行和只读查询方法
  - [x] 2.3: 编译验证

- [x] Task 3: 精简 remote.rs 和移除 DryRunContext
  - [x] 3.1: 移除 `fix_remote_names` 函数
  - [x] 3.2: 保留 `resolve_remote_name` 和 `diagnose_remote_names`
  - [x] 3.3: 移除 `DryRunContext`
  - [x] 3.4: 编译验证

- [x] Task 4: 重构 release.rs 使用三阶段流程
  - [x] 4.1: 使用 `GitContext::collect`
  - [x] 4.2: 构建 `ExecutionPlan`
  - [x] 4.3: dry_run 时调用 `plan.display()`
  - [x] 4.4: 移除旧函数
  - [x] 4.5: 编译验证

- [x] Task 5: 重构 sync.rs 使用三阶段流程
  - [x] 5.1: 使用 `GitContext::collect`
  - [x] 5.2: 构建 `ExecutionPlan`
  - [x] 5.3: dry_run 时调用 `plan.display()`
  - [x] 5.4: 编译验证

- [x] Task 6: 重构 doctor.rs 使用三阶段流程
  - [x] 6.1: 使用 `GitContext::collect`
  - [x] 6.2: 修复操作通过 `ExecutionPlan`
  - [x] 6.3: dry_run 时调用 `plan.display()`
  - [x] 6.4: 移除 `fix_remote_names` 调用
  - [x] 6.5: 编译验证

- [x] Task 7: 重构 branch.rs 使用三阶段流程
  - [x] 7.1: `branch list` 使用 `GitContext`
  - [x] 7.2: `branch clean` 构建 `ExecutionPlan`
  - [x] 7.3: `branch switch` 构建 `ExecutionPlan`
  - [x] 7.4: `branch rename` 构建 `ExecutionPlan`
  - [x] 7.5: 编译验证

- [x] Task 8: 重构 snap.rs 使用三阶段流程
  - [x] 8.1: `snap create` 构建 `ExecutionPlan`
  - [x] 8.2: `snap restore` 构建 `ExecutionPlan`
  - [x] 8.3: `snap list` 使用只读查询
  - [x] 8.4: 编译验证

- [x] Task 9: 重构 fork.rs 使用三阶段流程
  - [x] 9.1: `fork` 构建 `ExecutionPlan`
  - [x] 9.2: 编译验证

- [x] Task 10: 重构 gitlab.rs 使用三阶段流程
  - [x] 10.1: `gitlab clone` 构建 `ExecutionPlan`
  - [x] 10.2: 编译验证

- [x] Task 11: 重构 status.rs 使用 GitContext
  - [x] 11.1: 使用 `GitContext` 替代分散查询
  - [x] 11.2: 编译验证

- [x] Task 12: 全量编译和测试
  - [x] 12.1: `cargo check` 无错误
  - [x] 12.2: `cargo test` 通过

# Task Dependencies
- [Task 2] depends on [Task 1]
- [Task 3] depends on [Task 1]
- [Task 4] depends on [Task 1, Task 2]
- [Task 5] depends on [Task 1, Task 2]
- [Task 6] depends on [Task 1, Task 2, Task 3]
- [Task 7] depends on [Task 1, Task 2]
- [Task 8] depends on [Task 1, Task 2, Task 3]
- [Task 9] depends on [Task 1, Task 2]
- [Task 10] depends on [Task 1, Task 2]
- [Task 11] depends on [Task 1]
- [Task 12] depends on [Task 4, Task 5, Task 6, Task 7, Task 8, Task 9, Task 10, Task 11]
