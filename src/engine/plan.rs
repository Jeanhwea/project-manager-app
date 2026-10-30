//! 执行计划的解释器：按顺序执行 `ExecutionPlan` 里的各 Phase 步骤，
//! 处理 dry-run 预览、整体 deadline 分摊到单步超时、以及 Phase 级别的失败容错策略。

use crate::domain::git::GitCommandRunner;
use crate::domain::self_update::{SelfUpdateError, download_asset, install_binary};
use crate::error::{AppError, Result};
use crate::model::operation::{EditOperation, Operation, SelfUpdateOperation, ShellOperation};
use crate::model::plan::{
    DisplayMessage, ExecutionPlan, ExecutionResult, OperationError, Phase, Step,
};
use crate::utils::output;
use std::time::{Duration, Instant};

/// 不设时间上限地执行计划
pub fn run_plan(plan: &ExecutionPlan) -> Result<ExecutionResult> {
    run_plan_with_deadline(plan, None)
}

/// 执行计划，`deadline` 到了会中止后续步骤；dry-run 计划只打印不落地
pub fn run_plan_with_deadline(
    plan: &ExecutionPlan,
    deadline: Option<Instant>,
) -> Result<ExecutionResult> {
    if plan.dry_run() {
        // dry-run 不产生副作用，因此直接返回空结果，deadline 也一并忽略
        let count = plan.operation_count();
        output::dry_run_header(&format!("将要执行的操作 ({} 条):", count));
        display_plan(plan);
        return Ok(ExecutionResult::new());
    }

    render_messages(plan.messages());

    let runner = GitCommandRunner::new();
    let mut result = ExecutionResult::new();

    for phase in plan.phases() {
        if phase.is_empty() {
            continue;
        }
        output::section(&format!("▸ {}", phase.label()));

        if !run_phase(phase, &runner, &mut result, deadline) {
            return Ok(result);
        }
    }

    render_messages(plan.footer_messages());

    Ok(result)
}

/// 执行单个 Phase，返回 false 表示应停止整个计划的后续 Phase
fn run_phase(
    phase: &Phase,
    runner: &GitCommandRunner,
    result: &mut ExecutionResult,
    deadline: Option<Instant>,
) -> bool {
    for step in phase.steps() {
        if let Some(deadline) = deadline
            && Instant::now() >= deadline
        {
            result.add_error(OperationError::new("同步超时"));
            output::error("执行失败: 同步超时，跳过剩余操作");
            return false;
        }

        match step {
            Step::Op(op) => {
                let tolerate = phase.is_continue_on_error();
                // 剩余时间作为该步骤的子进程超时，双保险：既不无限等待，也不越过总 deadline
                let timeout = deadline.map(|d| d.saturating_duration_since(Instant::now()));
                if !run_op_step(op, runner, result, tolerate, timeout) && !tolerate {
                    return false;
                }
            }
            Step::Msg(msg) => render_message(msg),
        }
    }
    true
}

/// 执行一个步骤，返回 false 表示该步骤失败；是否继续由调用方按容错策略决定
fn run_op_step(
    op: &Operation,
    runner: &GitCommandRunner,
    result: &mut ExecutionResult,
    tolerate: bool,
    timeout: Option<Duration>,
) -> bool {
    if let Operation::Git(git_op) = op
        && let Some(reason) = git_op.should_skip()
    {
        output::skip(&reason);
        result.add_executed();
        return true;
    }

    output::command(&op.description());
    match execute_operation(op, runner, timeout) {
        Ok(()) => {
            result.add_executed();
            true
        }
        Err(e) => {
            if tolerate {
                output::warning(&format!("执行失败，已跳过: {}", first_line(&e.to_string())));
                // 容错模式下不计入 executed，也不记 error，后续阶段不受影响
                return false;
            }
            let hint = recovery_hint(op, result.executed_count());
            let description = if is_timeout(&e) {
                format!("{} (超时)", op.description())
            } else {
                op.description()
            };
            let error = OperationError::new(description).with_recovery_hint(hint);
            result.add_error(error);
            output::error(&format!("执行失败: {}", e));
            false
        }
    }
}

fn is_timeout(err: &AppError) -> bool {
    matches!(
        err,
        AppError::Git(crate::domain::git::GitError::Timeout { .. })
    )
}

// 错误信息常带多行 git 输出，回显时压缩成第一行非空内容
fn first_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or(text)
        .to_string()
}

/// 失败后给出人工补救提示；git 操作有专属文案，其余退化为"已完成 N 步"
fn recovery_hint(failed_op: &Operation, executed_count: usize) -> String {
    if let Operation::Git(git_op) = failed_op
        && let Some(text) = git_op.recovery_hint(executed_count)
    {
        return text;
    }
    format!("{} 个操作已完成", executed_count)
}

/// 只打印计划不做任何改动，用于 dry-run；计划为空时提示"无操作"
pub fn display_plan(plan: &ExecutionPlan) {
    let has_operations = plan.operation_count() > 0;
    if !has_operations && plan.messages().is_empty() && plan.footer_messages().is_empty() {
        output::skip("无操作");
        return;
    }

    render_messages(plan.messages());

    for phase in plan.phases() {
        if phase.is_empty() {
            continue;
        }
        output::section(&format!("▸ {}", phase.label()));
        for step in phase.steps() {
            match step {
                Step::Op(op) => output::dry_command(&op.description()),
                Step::Msg(msg) => render_message(msg),
            }
        }
    }

    render_messages(plan.footer_messages());
}

pub fn render_messages(messages: &[DisplayMessage]) {
    for msg in messages {
        render_message(msg);
    }
}

pub fn render_message(msg: &DisplayMessage) {
    match msg {
        DisplayMessage::Header { title } => output::header(title),
        DisplayMessage::Section { title } => output::section(title),
        DisplayMessage::Item { label, value } => output::item(label, value),
        DisplayMessage::Detail { label, value } => output::detail(label, value),
        DisplayMessage::Diff {
            file,
            old_start,
            new_start,
            old_lines,
            new_lines,
            old_count,
            new_count,
        } => {
            output::message(&format!("diff --git a/{} b/{}", file, file));
            output::message(&format!("--- a/{}", file));
            output::message(&format!("+++ b/{}", file));
            output::message(&format!(
                "@@ -{},{} +{},{} @@",
                old_start, old_count, new_start, new_count
            ));
            for line in old_lines {
                output::removed_line(line);
            }
            for line in new_lines {
                output::added_line(line);
            }
        }
        DisplayMessage::Success { msg } => output::success(msg),
        DisplayMessage::Warning { msg } => output::warning(msg),
        DisplayMessage::Skip { msg } => output::skip(msg),
        DisplayMessage::Blank => output::blank(),
    }
}

/// 按类型分发到各执行器
fn execute_operation(
    op: &Operation,
    runner: &GitCommandRunner,
    timeout: Option<Duration>,
) -> Result<()> {
    match op {
        Operation::Git(git_op) => git_op.execute(runner, timeout),
        Operation::Shell(shell_op) => execute_shell(shell_op, timeout),
        Operation::Edit(edit_op) => execute_edit(edit_op),
        Operation::SelfUpdate(update_op) => execute_self_update(update_op),
    }
}

// optional 的命令无论退出码如何都不算失败，只降级为告警
fn execute_shell(op: &ShellOperation, timeout: Option<Duration>) -> Result<()> {
    match op {
        ShellOperation::Run {
            program,
            args,
            dir,
            optional,
            ..
        } => {
            use crate::domain::runner::{CommandRunner, ExecutionContext, OutputMode};

            let mut ctx = ExecutionContext::new(program)
                .args(args.iter().cloned())
                .output_mode(OutputMode::Streaming)
                .timeout(timeout);

            if let Some(dir) = dir {
                ctx = ctx.working_dir(dir);
            }

            let runner = CommandRunner;
            let result = match runner.execute(&ctx) {
                Ok(r) => r,
                Err(e) => {
                    if *optional {
                        output::warning(&format!("无法执行 {}: {}", program, e));
                        return Ok(());
                    }
                    return Err(AppError::InvalidInput {
                        reason: format!("无法执行 {}: {}", program, e),
                    });
                }
            };

            if !result.success {
                if *optional {
                    output::warning(&format!(
                        "{} 执行失败 (exit code {})、已跳过继续执行",
                        program, result.exit_code
                    ));
                    return Ok(());
                }
                return Err(AppError::InvalidInput {
                    reason: format!("{} 执行失败 (exit code {})", program, result.exit_code),
                });
            }
        }
    }
    Ok(())
}

fn execute_edit(op: &EditOperation) -> Result<()> {
    match op {
        EditOperation::WriteFile { path, content, .. } => {
            crate::domain::editor::write_with_backup(path, content)?;
        }
        EditOperation::CopyDir { source, target, .. } => {
            copy_dir_recursive(std::path::Path::new(source), std::path::Path::new(target))?;
        }
    }
    Ok(())
}

fn copy_dir_recursive(source: &std::path::Path, target: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(target).map_err(|e| {
        AppError::Io(std::io::Error::new(
            e.kind(),
            format!("创建目录 {} 失败: {}", target.display(), e),
        ))
    })?;

    for entry in std::fs::read_dir(source).map_err(AppError::Io)? {
        let entry = entry.map_err(AppError::Io)?;
        let src_path = entry.path();
        let dst_path = target.join(entry.file_name());

        if src_path.is_dir() {
            // 不搬模板仓库自身的 .git：fork 出来的项目应当有自己的历史
            if entry.file_name() == ".git" {
                continue;
            }
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(AppError::Io)?;
        }
    }

    Ok(())
}

/// 下载发布资产后原地替换当前可执行文件；中途失败会留下 SelfUpdateError 供上层提示手动安装
fn execute_self_update(op: &SelfUpdateOperation) -> Result<()> {
    match op {
        SelfUpdateOperation::DownloadAndInstall {
            api_url,
            browser_url,
            asset_name,
            ..
        } => {
            output::info(&format!("下载 {}...", asset_name));
            let data = download_asset(api_url, browser_url, asset_name)?;
            output::success("下载完成");

            let current_exe = std::env::current_exe()
                .map_err(|e| SelfUpdateError::CurrentExePath { source: e })?;
            install_binary(&data, asset_name, &current_exe)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::operation::ShellOperation;
    use crate::model::plan::Phase;

    // 依赖系统自带的 echo，测试只关心步骤推进与超时行为，不关心命令输出
    fn echo_plan() -> ExecutionPlan {
        let mut plan = ExecutionPlan::new();
        let mut phase = Phase::new("测试阶段");
        phase.add(ShellOperation::Run {
            program: "echo".to_string(),
            args: vec!["hello".to_string()],
            dir: None,
            description: "echo hello".to_string(),
            optional: false,
        });
        plan.add_phase(phase);
        plan
    }

    #[test]
    fn test_run_plan_without_deadline_runs_all_steps() {
        let result = run_plan(&echo_plan()).unwrap();
        assert_eq!(result.executed_count(), 1);
        assert!(result.is_success());
    }

    #[test]
    fn test_deadline_in_future_runs_all_steps() {
        let deadline = Instant::now() + Duration::from_secs(60);
        let result = run_plan_with_deadline(&echo_plan(), Some(deadline)).unwrap();
        assert_eq!(result.executed_count(), 1);
        assert!(result.is_success());
    }

    #[test]
    fn test_deadline_already_passed_skips_remaining_steps() {
        let deadline = Instant::now() - Duration::from_secs(1);
        let result = run_plan_with_deadline(&echo_plan(), Some(deadline)).unwrap();
        assert_eq!(result.executed_count(), 0);
        assert!(!result.is_success());
        assert_eq!(result.errors()[0].description(), "同步超时");
    }

    #[test]
    fn test_dry_run_ignores_deadline() {
        let plan = echo_plan().with_dry_run(true);
        let deadline = Instant::now() - Duration::from_secs(1);
        let result = run_plan_with_deadline(&plan, Some(deadline)).unwrap();
        assert!(result.is_success());
    }
}
