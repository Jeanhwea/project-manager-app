use crate::domain::git::GitCommandRunner;
use crate::model::plan::{
    EditOperation, ExecutionPlan, GitOperation, MessageOperation, Operation, ShellOperation,
};
use crate::utils::output::Output;
use std::path::Path;

pub fn run_plan(plan: &ExecutionPlan) -> anyhow::Result<()> {
    if plan.dry_run {
        Output::dry_run_header("将要执行的操作:");
        display_plan(plan);
        return Ok(());
    }

    for op in &plan.operations {
        match op {
            Operation::Message(msg_op) => execute_message(msg_op),
            _ => {
                Output::cmd(&op.description());
                execute_operation(op)?;
            }
        }
    }
    Ok(())
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
            Output::detail(&format!("L{} -", line_num), old_content);
            Output::detail(&format!("L{} +", line_num), new_content);
        }
        MessageOperation::Success { msg } => Output::success(msg),
        MessageOperation::Warning { msg } => Output::warning(msg),
        MessageOperation::Info { msg } => Output::info(msg),
        MessageOperation::Skip { msg } => Output::skip(msg),
        MessageOperation::Blank => Output::blank(),
    }
}

fn execute_operation(op: &Operation) -> anyhow::Result<()> {
    match op {
        Operation::Git(git_op) => execute_git(git_op),
        Operation::Shell(shell_op) => execute_shell(shell_op),
        Operation::Edit(edit_op) => execute_edit(edit_op),
        Operation::Message(_) => Ok(()),
    }
}

fn execute_git(op: &GitOperation) -> anyhow::Result<()> {
    let runner = GitCommandRunner::new();
    match op {
        GitOperation::Init { dir } => runner.execute_with_success(&["init"], Some(dir))?,
        GitOperation::Clone { url, dir } => runner.execute_streaming(&["clone", url], dir)?,
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

fn execute_shell(op: &ShellOperation) -> anyhow::Result<()> {
    match op {
        ShellOperation::Run {
            program, args, dir, ..
        } => {
            #[cfg(target_os = "windows")]
            let result = {
                let full_args: Vec<String> = std::iter::once("/c".to_string())
                    .chain(std::iter::once(program.clone()))
                    .chain(args.iter().cloned())
                    .collect();
                let full_args_ref: Vec<&str> = full_args.iter().map(|s| s.as_str()).collect();
                let mut cmd = std::process::Command::new("cmd");
                cmd.args(&full_args_ref);
                if let Some(dir) = dir {
                    cmd.current_dir(dir);
                }
                cmd.status()
            };
            #[cfg(not(target_os = "windows"))]
            let result = {
                let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                let mut cmd = std::process::Command::new(program);
                cmd.args(&args_ref);
                if let Some(dir) = dir {
                    cmd.current_dir(dir);
                }
                cmd.status()
            };

            let status = result.map_err(|e| anyhow::anyhow!("无法执行 {}: {}", program, e))?;
            if !status.success() {
                return Err(anyhow::anyhow!("{} 执行失败", program));
            }
        }
    }
    Ok(())
}

fn execute_edit(op: &EditOperation) -> anyhow::Result<()> {
    match op {
        EditOperation::WriteFile { path, content, .. } => {
            crate::domain::editor::write_with_backup(path, content)?;
        }
    }
    Ok(())
}
