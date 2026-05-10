# Tasks

- [x] Task 1: 创建 model 层
  - [x] 1.1: 创建 `src/model/mod.rs`
  - [x] 1.2: 创建 `src/model/git.rs`，迁移 Remote/Branch/Tag/GitContext
  - [x] 1.3: 创建 `src/model/plan.rs`，迁移 GitOperation/ExecutionPlan
  - [x] 1.4: 编译验证

- [x] Task 2: 创建 control 层
  - [x] 2.1: 创建 `src/control/mod.rs`
  - [x] 2.2: 创建 `src/control/context.rs`，迁移 GitContext 收集逻辑
  - [x] 2.3: 创建 `src/control/plan.rs`，迁移 ExecutionPlan execute 逻辑
  - [x] 2.4: 创建 `src/control/remote.rs`，迁移 resolve_remote_name/diagnose_remote_names
  - [x] 2.5: 编译验证

- [x] Task 3: 精简 domain/git 层
  - [x] 3.1: 移除 `domain/git/executor.rs`
  - [x] 3.2: 移除 `domain/git/models.rs`
  - [x] 3.3: 移除 `domain/git/remote.rs`
  - [x] 3.4: 更新 `domain/git/mod.rs`，只保留 command/repository
  - [x] 3.5: 更新 `domain/git/command.rs`，返回类型改为 model 层结构体
  - [x] 3.6: 编译验证

- [x] Task 4: 更新 commands 层引用
  - [x] 4.1: 更新 release.rs 使用 control/model 层
  - [x] 4.2: 更新 sync.rs 使用 control/model 层
  - [x] 4.3: 更新 doctor.rs 使用 control/model 层
  - [x] 4.4: 更新 branch.rs 使用 control/model 层
  - [x] 4.5: 更新 snap.rs 使用 control/model 层
  - [x] 4.6: 更新 fork.rs 使用 control/model 层
  - [x] 4.7: 更新 gitlab.rs 使用 control/model 层
  - [x] 4.8: 更新 status.rs 使用 control/model 层
  - [x] 4.9: 编译验证

- [x] Task 5: 全量编译和测试
  - [x] 5.1: `cargo check` 无错误
  - [x] 5.2: `cargo test` 通过

# Task Dependencies
- [Task 2] depends on [Task 1]
- [Task 3] depends on [Task 1, Task 2]
- [Task 4] depends on [Task 1, Task 2, Task 3]
- [Task 5] depends on [Task 4]
