use crate::domain::git::GitCommandRunner;
use crate::model::plan::{ExecutionPlan, GitOperation};
use crate::utils::output::Output;
use std::path::Path;

pub fn run_plan(plan: &ExecutionPlan) -> anyhow::Result<()> {
    if plan.dry_run {
        Output::dry_run_header("将要执行的操作:");
        display_plan(plan);
        return Ok(());
    }

    let runner = GitCommandRunner::new();
    for op in &plan.operations {
        Output::cmd(&op.description());
        execute_operation(&runner, op)?;
    }
    Ok(())
}

pub fn display_plan(plan: &ExecutionPlan) {
    if plan.operations.is_empty() {
        Output::skip("无操作");
        return;
    }
    for op in &plan.operations {
        Output::message(&op.description());
    }
}

fn execute_operation(runner: &GitCommandRunner, op: &GitOperation) -> anyhow::Result<()> {
    match op {
        GitOperation::Init { dir } => runner.execute_with_success(&["init"], Some(dir))?,
        GitOperation::Clone { url, dir } => runner.execute_streaming(&["clone", url], dir)?,
        GitOperation::Add { path } => runner.execute_with_success(&["add", path], None)?,
        GitOperation::Commit { message } => {
            runner.execute_with_success(&["commit", "-m", message], None)?
        }
        GitOperation::CreateTag { tag } => runner.execute_with_success(&["tag", tag], None)?,
        GitOperation::PushTag { remote, tag } => {
            runner.execute_with_success(&["push", remote, tag], None)?
        }
        GitOperation::PushBranch { remote, branch } => {
            runner.execute_with_success(&["push", remote, branch], None)?
        }
        GitOperation::PushAll { remote } => {
            runner.execute_with_success(&["push", "--all", remote], None)?
        }
        GitOperation::PushTags { remote } => {
            runner.execute_with_success(&["push", "--tags", remote], None)?
        }
        GitOperation::Pull { remote, branch } => {
            runner.execute_streaming(&["pull", remote, branch], Path::new("."))?
        }
        GitOperation::Checkout { ref_name } => {
            runner.execute_streaming(&["checkout", ref_name], Path::new("."))?
        }
        GitOperation::BranchDelete { branch } => {
            runner.execute_with_success(&["branch", "-d", branch], None)?
        }
        GitOperation::BranchRename { old, new } => {
            runner.execute_streaming(&["branch", "-m", old, new], Path::new("."))?
        }
        GitOperation::RemoteDelete { remote, branch } => {
            runner.execute_with_success(&["push", remote, "--delete", branch], None)?
        }
        GitOperation::RemoteRename { old, new } => {
            runner.execute_with_success(&["remote", "rename", old, new], Some(Path::new(".")))?
        }
        GitOperation::RemotePrune { remote } => {
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
        GitOperation::Gc => runner.execute_with_success(&["gc", "--aggressive"], None)?,
        GitOperation::Custom { args, .. } => {
            let args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            runner.execute(&args, Some(Path::new(".")))?;
        }
    }
    Ok(())
}
