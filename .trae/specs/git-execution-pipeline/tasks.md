# Tasks

- [ ] Task 1: 实现 GitContext 和 ExecutionPlan
  - [ ] 1.1: 创建 `src/domain/git/executor.rs`，定义 `GitContext`（复用 `GitCommandRunner` 的查询方法收集上下文）
  - [ ] 1.2: 定义 `GitOperation` 枚举，覆盖 add/commit/create_tag/push_tag/push_branch/push_all/push_tags/pull/checkout/remote_rename/remote_prune/set_upstream
  - [ ] 1.3: 定义 `ExecutionPlan`，支持 `add(op)` / `dry_run(bool)` / `display()` / `execute()`
  - [ ] 1.4: 在 `mod.rs` 中注册 `executor` 模块
  - [ ] 1.5: 编译验证

- [ ] Task 2: 精简 GitCommandRunner
  - [ ] 2.1: 移除 command.rs 中已迁移到 GitOperation 的高级写方法（add/commit/create_tag/push_tag/push_branch/push_all/push_tags/pull/checkout/describe_tags/rev_parse/set_upstream/remote_prune/rename_remote/diff_cached/log_oneline/rev_list_count/show_toplevel）
  - [ ] 2.2: 保留底层 execute/execute_with_success/execute_streaming/execute_raw 和只读查询方法
  - [ ] 2.3: 编译验证

- [ ] Task 3: 精简 remote.rs
  - [ ] 3.1: 移除 `fix_remote_names` 函数，remote 修复通过 ExecutionPlan 实现
  - [ ] 3.2: 保留 `resolve_remote_name` 和 `diagnose_remote_names`
  - [ ] 3.3: 编译验证

- [ ] Task 4: 重构 release.rs 使用三阶段流程
  - [ ] 4.1: 使用 `GitContext::collect` 替代分散的 git 查询
  - [ ] 4.2: 构建 `ExecutionPlan` 替代直接调用 git 命令
  - [ ] 4.3: dry_run 时调用 `plan.display()` 而非 `plan.execute()`
  - [ ] 4.4: 移除 `get_remotes` / `push_to_remotes` / `print_push_plan` 等旧函数
  - [ ] 4.5: 编译验证

- [ ] Task 5: 重构 sync.rs 使用三阶段流程
  - [ ] 5.1: 使用 `GitContext::collect` 收集 remotes 和 current_branch
  - [ ] 5.2: 构建 `ExecutionPlan` 替代直接调用 git 命令
  - [ ] 5.3: dry_run 时调用 `plan.display()` 而非 `plan.execute()`
  - [ ] 5.4: 编译验证

- [ ] Task 6: 重构 doctor.rs 使用三阶段流程
  - [ ] 6.1: 使用 `GitContext::collect` 替代分散的 git 查询
  - [ ] 6.2: 修复操作通过 `ExecutionPlan` 实现，替代 `fix_issues` 中的直接调用
  - [ ] 6.3: dry_run 时调用 `plan.display()` 而非 `plan.execute()`
  - [ ] 6.4: 移除 `fix_remote_names` 调用，改用 ExecutionPlan
  - [ ] 6.5: 编译验证

- [ ] Task 7: 全量编译和测试
  - [ ] 7.1: `cargo check` 无错误
  - [ ] 7.2: `cargo test` 通过

# Task Dependencies
- [Task 2] depends on [Task 1]
- [Task 3] depends on [Task 1]
- [Task 4] depends on [Task 1, Task 2]
- [Task 5] depends on [Task 1, Task 2]
- [Task 6] depends on [Task 1, Task 2, Task 3]
- [Task 7] depends on [Task 4, Task 5, Task 6]
