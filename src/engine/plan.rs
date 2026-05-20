use crate::domain::git::GitCommandRunner;
use crate::domain::selfupdate::{download_asset, install_binary};
use crate::error::{AppError, Result};
use crate::model::operation::{EditOperation, Operation, SelfUpdateOperation, ShellOperation};
use crate::model::plan::{DisplayMessage, ExecutionPlan, ExecutionResult, OperationError, Step};
use crate::utils::output;

pub fn run_plan(plan: &ExecutionPlan) -> Result<ExecutionResult> {
    if plan.dry_run() {
        let count = plan.operation_count();
        output::dry_run_header(&format!("将要执行的操作 ({} 条):", count));
        display_plan(plan);
        return Ok(ExecutionResult::new());
    }

    render_messages(plan.messages());

    let mut result = ExecutionResult::new();
    let runner = GitCommandRunner::new();

    for phase in plan.phases() {
        if phase.is_empty() {
            continue;
        }

        output::section(&format!("▸ {}", phase.label()));

        for step in phase.steps() {
            match step {
                Step::Op(op) => {
                    if let Operation::Git(git_op) = op
                        && let Some(reason) = git_op.should_skip()
                    {
                        output::skip(&reason);
                        result.add_executed();
                        continue;
                    }
                    output::command(&op.description());
                    match execute_operation(op, &runner) {
                        Ok(()) => {
                            result.add_executed();
                        }
                        Err(e) => {
                            let hint = recovery_hint(op, result.executed_count());
                            let error =
                                OperationError::new(op.description()).with_recovery_hint(hint);
                            result.add_error(error);
                            output::error(&format!("执行失败: {}", e));
                            return Ok(result);
                        }
                    }
                }
                Step::Msg(msg) => {
                    render_message(msg);
                }
            }
        }
    }

    Ok(result)
}

fn recovery_hint(failed_op: &Operation, executed_count: usize) -> String {
    if let Operation::Git(git_op) = failed_op
        && let Some(text) = git_op.recovery_hint(executed_count)
    {
        return text;
    }
    format!("{} 个操作已完成", executed_count)
}

pub fn display_plan(plan: &ExecutionPlan) {
    let has_operations = plan.operation_count() > 0;
    if !has_operations && plan.messages().is_empty() {
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

fn execute_operation(op: &Operation, runner: &GitCommandRunner) -> Result<()> {
    match op {
        Operation::Git(git_op) => git_op.execute(runner),
        Operation::Shell(shell_op) => execute_shell(shell_op),
        Operation::Edit(edit_op) => execute_edit(edit_op),
        Operation::SelfUpdate(update_op) => execute_self_update(update_op),
    }
}

fn execute_shell(op: &ShellOperation) -> Result<()> {
    match op {
        ShellOperation::Run {
            program, args, dir, ..
        } => {
            use crate::domain::runner::{CommandRunner, ExecutionContext, OutputMode};

            let mut ctx = ExecutionContext::new(program)
                .args(args.iter().cloned())
                .output_mode(OutputMode::Streaming);

            if let Some(dir) = dir {
                ctx = ctx.working_dir(dir);
            }

            let runner = CommandRunner;
            let result = runner
                .execute(&ctx)
                .map_err(|e| AppError::invalid_input(format!("无法执行 {}: {}", program, e)))?;

            if !result.success {
                return Err(AppError::invalid_input(format!(
                    "{} 执行失败 (exit code {})",
                    program, result.exit_code
                )));
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

fn execute_self_update(op: &SelfUpdateOperation) -> Result<()> {
    match op {
        SelfUpdateOperation::DownloadAndInstall {
            api_url,
            browser_url,
            asset_name,
            ..
        } => {
            output::info(&format!("下载 {}...", asset_name));
            let data = download_asset(api_url, browser_url, asset_name)
                .map_err(|e| AppError::self_update(format!("下载资源失败: {}", e)))?;
            output::success("下载完成");

            let current_exe = std::env::current_exe().map_err(|e| {
                AppError::self_update(format!("无法获取当前可执行文件路径: {}", e))
            })?;
            install_binary(&data, asset_name, &current_exe)
                .map_err(|e| AppError::self_update(format!("安装二进制文件失败: {}", e)))?;
        }
    }
    Ok(())
}
