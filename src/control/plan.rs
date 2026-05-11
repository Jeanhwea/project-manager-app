use crate::domain::git::GitCommandRunner;
use crate::domain::selfupdate::{download_asset, install_binary};
use crate::error::{AppError, Result};
use crate::model::plan::{
    EditOperation, ExecutionPlan, GitOperation, MessageOperation, Operation, SelfUpdateOperation,
    ShellOperation,
};
use crate::utils::output::Output;
use std::path::Path;

pub fn run_plan(plan: &ExecutionPlan) -> Result<()> {
    if plan.dry_run {
        Output::dry_run_header("将要执行的操作:");
        display_plan(plan);
        return Ok(());
    }

    let mut executed = Vec::new();
    for op in &plan.operations {
        match op {
            Operation::Message(msg_op) => execute_message(msg_op),
            _ => {
                Output::cmd(&op.description());
                if let Err(e) = execute_operation(op) {
                    Output::error(&format!("执行失败: {}", e));
                    emit_recovery_hints(&executed, op);
                    return Err(e);
                }
                executed.push(op.description());
            }
        }
    }
    Ok(())
}

fn emit_recovery_hints(executed: &[String], failed_op: &Operation) {
    if executed.is_empty() {
        return;
    }

    Output::blank();
    Output::warning("恢复指引:");

    match failed_op {
        Operation::Git(git_op) => match git_op {
            GitOperation::PushTag { remote, tag } => {
                Output::detail(
                    "提示",
                    &format!(
                        "tag {} 已创建但未推送，请手动执行: git push {} {}",
                        tag, remote, tag
                    ),
                );
            }
            GitOperation::PushBranch { remote, branch } => {
                Output::detail(
                    "提示",
                    &format!(
                        "commit 已创建但未推送，请手动执行: git push {} {}",
                        remote, branch
                    ),
                );
            }
            GitOperation::PushAll { remote } => {
                Output::detail(
                    "提示",
                    &format!(
                        "commit 已创建但未推送，请手动执行: git push --all {}",
                        remote
                    ),
                );
            }
            GitOperation::PushTags { remote } => {
                Output::detail(
                    "提示",
                    &format!("tag 已创建但未推送，请手动执行: git push --tags {}", remote),
                );
            }
            _ => {
                Output::detail("已执行", &format!("{} 个操作已完成", executed.len()));
            }
        },
        _ => {
            Output::detail("已执行", &format!("{} 个操作已完成", executed.len()));
        }
    }
}

pub fn display_plan(plan: &ExecutionPlan) {
    if plan.operations.is_empty() {
        Output::skip("无操作");
        return;
    }
    for op in &plan.operations {
        match op {
            Operation::Message(msg_op) => execute_message(msg_op),
            _ => Output::message(&op.description()),
        }
    }
}

fn execute_message(op: &MessageOperation) {
    use colored::Colorize;
    match op {
        MessageOperation::Header { title } => Output::header(title),
        MessageOperation::Section { title } => Output::section(title),
        MessageOperation::Item { label, value } => Output::item(label, value),
        MessageOperation::Detail { label, value } => Output::detail(label, value),
        MessageOperation::Diff {
            file: _,
            line_num,
            old_content,
            new_content,
        } => {
            println!("    L{} {}", line_num, format!("- {}", old_content).red());
            println!("    L{} {}", line_num, format!("+ {}", new_content).green());
        }
        MessageOperation::Success { msg } => Output::success(msg),
        MessageOperation::Warning { msg } => Output::warning(msg),
        MessageOperation::Skip { msg } => Output::skip(msg),
        MessageOperation::Blank => Output::blank(),
    }
}

fn execute_operation(op: &Operation) -> Result<()> {
    match op {
        Operation::Git(git_op) => execute_git(git_op),
        Operation::Shell(shell_op) => execute_shell(shell_op),
        Operation::Edit(edit_op) => execute_edit(edit_op),
        Operation::SelfUpdate(update_op) => execute_self_update(update_op),
        Operation::Message(_) => Ok(()),
    }
}

fn execute_git(op: &GitOperation) -> Result<()> {
    let runner = GitCommandRunner::new();
    match op {
        GitOperation::Init { dir } => runner.execute_with_success(&["init"], Some(dir))?,
        GitOperation::Clone { url, dir } => runner
            .execute_streaming(&["clone", url, dir.to_str().unwrap_or(".")], Path::new("."))?,
        GitOperation::Add { path } => runner.execute_with_success(&["add", path], None)?,
        GitOperation::Commit { message } => {
            runner.execute_with_success(&["commit", "-m", message], None)?
        }
        GitOperation::CreateTag { tag } => runner.execute_with_success(&["tag", tag], None)?,
        GitOperation::PushTag { remote, tag } => {
            runner.execute_streaming(&["push", remote, tag], Path::new("."))?
        }
        GitOperation::PushBranch { remote, branch } => {
            runner.execute_streaming(&["push", remote, branch], Path::new("."))?
        }
        GitOperation::PushAll { remote } => {
            runner.execute_streaming(&["push", "--all", remote], Path::new("."))?
        }
        GitOperation::PushTags { remote } => {
            runner.execute_streaming(&["push", "--tags", remote], Path::new("."))?
        }
        GitOperation::Pull { remote, branch } => {
            runner.execute_streaming(&["pull", remote, branch], Path::new("."))?
        }
        GitOperation::Checkout { ref_name } => {
            runner.execute_streaming(&["checkout", ref_name], Path::new("."))?
        }
        GitOperation::DeleteBranch { branch } => {
            runner.execute_with_success(&["branch", "-d", branch], None)?
        }
        GitOperation::RenameBranch { old, new } => {
            runner.execute_streaming(&["branch", "-m", old, new], Path::new("."))?
        }
        GitOperation::DeleteRemoteBranch { remote, branch } => {
            runner.execute_streaming(&["push", remote, "--delete", branch], Path::new("."))?
        }
        GitOperation::RenameRemote { old, new } => {
            runner.execute_with_success(&["remote", "rename", old, new], Some(Path::new(".")))?
        }
        GitOperation::PruneRemote { remote } => {
            runner.execute_with_success(&["remote", "prune", remote], Some(Path::new(".")))?
        }
        GitOperation::SetUpstream { remote, branch } => runner.execute_with_success(
            &[
                "branch",
                "--set-upstream-to",
                &format!("{}/{}", remote, branch),
            ],
            Some(Path::new(".")),
        )?,
        GitOperation::Gc => runner.execute_streaming(&["gc", "--aggressive"], Path::new("."))?,
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
