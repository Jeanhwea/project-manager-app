use crate::domain::git::GitCommandRunner;
use crate::domain::selfupdate::{download_asset, install_binary};
use crate::error::{AppError, Result};
use crate::model::plan::{
    DisplayMessage, EditOperation, ExecutionPlan, ExecutionResult, GitOperation, Operation,
    OperationError, SelfUpdateOperation, ShellOperation,
};
use crate::utils::output::Output;

/// Execute an ExecutionPlan, returning a structured ExecutionResult.
pub fn run_plan(plan: &ExecutionPlan) -> Result<ExecutionResult> {
    if plan.dry_run() {
        let count = plan.operation_count();
        Output::dry_run_header(&format!("将要执行的操作 ({} 条):", count));
        display_plan(plan);
        return Ok(ExecutionResult::new());
    }

    // Render pre-execution messages
    render_messages(plan.messages());

    let mut result = ExecutionResult::new();

    for phase in plan.phases() {
        if !phase.is_empty() {
            Output::section(&format!("▸ {}", phase.label()));
        }

        for op in phase.operations() {
            Output::cmd(&op.description());
            match execute_operation(op) {
                Ok(()) => {
                    result.add_executed(crate::model::plan::ExecutedOperation::new(
                        op.description(),
                        phase.label(),
                    ));
                }
                Err(e) => {
                    let error = OperationError::new(op.description(), phase.label())
                        .with_recovery_hint(generate_recovery_hint(&result, op));
                    result.add_error(error);
                    Output::error(&format!("执行失败: {}", e));
                    return Ok(result);
                }
            }
        }
    }

    Ok(result)
}

fn generate_recovery_hint(result: &ExecutionResult, failed_op: &Operation) -> String {
    match failed_op {
        Operation::Git(git_op) => match git_op {
            GitOperation::PushTag { remote, tag, .. } => {
                format!(
                    "tag {} 已创建但未推送，请手动执行: git push {} {}",
                    tag, remote, tag
                )
            }
            GitOperation::PushBranch { remote, branch, .. } => {
                format!(
                    "commit 已创建但未推送，请手动执行: git push {} {}",
                    remote, branch
                )
            }
            GitOperation::PushAll { remote, .. } => {
                format!(
                    "commit 已创建但未推送，请手动执行: git push --all {}",
                    remote
                )
            }
            GitOperation::PushTags { remote, .. } => {
                format!("tag 已创建但未推送，请手动执行: git push --tags {}", remote)
            }
            _ => format!("{} 个操作已完成", result.executed().len()),
        },
        _ => format!("{} 个操作已完成", result.executed().len()),
    }
}

pub fn display_plan(plan: &ExecutionPlan) {
    let has_operations = plan.operation_count() > 0;
    if !has_operations && plan.messages().is_empty() {
        Output::skip("无操作");
        return;
    }

    // Render messages
    render_messages(plan.messages());

    // Render phases and their operations
    for phase in plan.phases() {
        if phase.is_empty() {
            continue;
        }
        Output::section(&format!("▸ {}", phase.label()));
        for op in phase.operations() {
            Output::dry_cmd(&op.description());
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
        DisplayMessage::Header { title } => Output::header(title),
        DisplayMessage::Section { title } => Output::section(title),
        DisplayMessage::Item { label, value } => Output::item(label, value),
        DisplayMessage::Detail { label, value } => Output::detail(label, value),
        DisplayMessage::Diff {
            file,
            old_start,
            new_start,
            old_lines,
            new_lines,
            old_count,
            new_count,
        } => {
            Output::message(&format!("diff --git a/{} b/{}", file, file));
            Output::message(&format!("--- a/{}", file));
            Output::message(&format!("+++ b/{}", file));
            Output::message(&format!(
                "@@ -{},{} +{},{} @@",
                old_start, old_count, new_start, new_count
            ));
            for line in old_lines {
                Output::diff_old(line);
            }
            for line in new_lines {
                Output::diff_new(line);
            }
        }
        DisplayMessage::Success { msg } => Output::success(msg),
        DisplayMessage::Warning { msg } => Output::warning(msg),
        DisplayMessage::Skip { msg } => Output::skip(msg),
        DisplayMessage::Blank => Output::blank(),
    }
}

fn execute_operation(op: &Operation) -> Result<()> {
    match op {
        Operation::Git(git_op) => execute_git(git_op),
        Operation::Shell(shell_op) => execute_shell(shell_op),
        Operation::Edit(edit_op) => execute_edit(edit_op),
        Operation::SelfUpdate(update_op) => execute_self_update(update_op),
    }
}

fn execute_git(op: &GitOperation) -> Result<()> {
    let runner = GitCommandRunner::new();

    match op {
        GitOperation::Init { working_dir } => {
            runner.execute_with_success(&["init"], Some(working_dir.as_path()))?
        }
        GitOperation::Clone {
            url,
            target_dir,
            working_dir,
        } => {
            if target_dir.exists() {
                Output::skip(&format!("目录 {} 已存在，跳过克隆", target_dir.display()));
                return Ok(());
            }
            runner.execute_streaming(
                &["clone", url, target_dir.to_str().unwrap_or(".")],
                working_dir.as_path(),
            )?
        }
        GitOperation::Add { path, working_dir } => {
            runner.execute_with_success(&["add", path], Some(working_dir.as_path()))?
        }
        GitOperation::Commit {
            message,
            working_dir,
        } => runner
            .execute_with_success(&["commit", "-m", message], Some(working_dir.as_path()))?,
        GitOperation::CreateTag { tag, working_dir } => {
            runner.execute_with_success(&["tag", tag], Some(working_dir.as_path()))?
        }
        GitOperation::PushTag {
            remote,
            tag,
            working_dir,
        } => runner.execute_streaming(&["push", remote, tag], working_dir.as_path())?,
        GitOperation::PushBranch {
            remote,
            branch,
            working_dir,
        } => runner.execute_streaming(&["push", remote, branch], working_dir.as_path())?,
        GitOperation::PushAll {
            remote,
            working_dir,
        } => runner.execute_streaming(&["push", "--all", remote], working_dir.as_path())?,
        GitOperation::PushTags {
            remote,
            working_dir,
        } => runner.execute_streaming(&["push", "--tags", remote], working_dir.as_path())?,
        GitOperation::Pull {
            remote,
            branch,
            working_dir,
        } => runner.execute_streaming(&["pull", remote, branch], working_dir.as_path())?,
        GitOperation::PullDefault { working_dir } => {
            runner.execute_streaming(&["pull"], working_dir.as_path())?
        }
        GitOperation::Checkout {
            ref_name,
            working_dir,
        } => runner.execute_streaming(&["checkout", ref_name], working_dir.as_path())?,
        GitOperation::DeleteBranch {
            branch,
            working_dir,
        } => {
            runner.execute_with_success(&["branch", "-d", branch], Some(working_dir.as_path()))?
        }
        GitOperation::RenameBranch {
            old,
            new,
            working_dir,
        } => runner.execute_streaming(&["branch", "-m", old, new], working_dir.as_path())?,
        GitOperation::DeleteRemoteBranch {
            remote,
            branch,
            working_dir,
        } => runner
            .execute_streaming(&["push", remote, "--delete", branch], working_dir.as_path())?,
        GitOperation::RenameRemote {
            old,
            new,
            working_dir,
        } => runner
            .execute_with_success(&["remote", "rename", old, new], Some(working_dir.as_path()))?,
        GitOperation::PruneRemote {
            remote,
            working_dir,
        } => runner
            .execute_with_success(&["remote", "prune", remote], Some(working_dir.as_path()))?,
        GitOperation::SetUpstream {
            remote,
            branch,
            working_dir,
        } => runner.execute_with_success(
            &[
                "branch",
                "--set-upstream-to",
                &format!("{}/{}", remote, branch),
            ],
            Some(working_dir.as_path()),
        )?,
        GitOperation::Gc { working_dir } => {
            runner.execute_streaming(&["gc", "--aggressive"], working_dir.as_path())?
        }
    }
    Ok(())
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
            Output::info(&format!("下载 {}...", asset_name));
            let data = download_asset(api_url, browser_url, asset_name)
                .map_err(|e| AppError::self_update(format!("下载资源失败: {}", e)))?;
            Output::success("下载完成");

            let current_exe = std::env::current_exe().map_err(|e| {
                AppError::self_update(format!("无法获取当前可执行文件路径: {}", e))
            })?;
            install_binary(&data, asset_name, &current_exe)
                .map_err(|e| AppError::self_update(format!("安装二进制文件失败: {}", e)))?;
        }
    }
    Ok(())
}
