use super::command::GitCommandRunner;
use super::models::{Branch, Remote, Tag};
use crate::utils::output::Output;
use std::path::{Path, PathBuf};

pub struct GitContext {
    pub root: PathBuf,
    pub current_branch: String,
    pub remotes: Vec<Remote>,
    pub branches: Vec<Branch>,
    pub tags: Vec<Tag>,
    pub has_uncommitted_changes: bool,
}

impl GitContext {
    pub fn collect(repo_path: &Path) -> anyhow::Result<Self> {
        let runner = GitCommandRunner::new();

        let root = runner.execute(&["rev-parse", "--show-toplevel"], Some(repo_path))?;
        let root = PathBuf::from(root);

        let current_branch = runner.get_current_branch(&root)?;
        let remotes = runner.get_all_remotes(&root)?;
        let branches = runner.get_all_branches(&root)?;
        let tags = runner.get_all_tags(&root)?;
        let has_uncommitted_changes = runner.has_uncommitted_changes(&root)?;

        Ok(Self {
            root,
            current_branch,
            remotes,
            branches,
            tags,
            has_uncommitted_changes,
        })
    }

    pub fn remote_names(&self) -> Vec<&str> {
        self.remotes.iter().map(|r| r.name.as_str()).collect()
    }

    pub fn has_remote(&self, name: &str) -> bool {
        self.remotes.iter().any(|r| r.name == name)
    }

    pub fn has_tag(&self, name: &str) -> bool {
        self.tags.iter().any(|t| t.name == name)
    }

    pub fn local_branches(&self) -> Vec<&Branch> {
        Branch::local_branches(&self.branches)
    }

    pub fn remote_branches(&self) -> Vec<&Branch> {
        Branch::remote_branches(&self.branches)
    }
}

#[derive(Debug, Clone)]
pub enum GitOperation {
    Init { dir: PathBuf },
    Clone { url: String, dir: PathBuf },
    Add { path: String },
    Commit { message: String },
    CreateTag { tag: String },
    PushTag { remote: String, tag: String },
    PushBranch { remote: String, branch: String },
    PushAll { remote: String },
    PushTags { remote: String },
    Pull { remote: String, branch: String },
    Checkout { ref_name: String },
    BranchDelete { branch: String },
    BranchRename { old: String, new: String },
    RemoteDelete { remote: String, branch: String },
    RemoteRename { old: String, new: String },
    RemotePrune { remote: String },
    SetUpstream { remote: String, branch: String },
    Gc,
    Custom { args: Vec<String>, description: String },
}

impl GitOperation {
    pub fn description(&self) -> String {
        match self {
            GitOperation::Init { .. } => "git init".to_string(),
            GitOperation::Clone { url, dir } => {
                format!("git clone {} {}", url, dir.display())
            }
            GitOperation::Add { path } => format!("git add {}", path),
            GitOperation::Commit { message } => format!("git commit -m \"{}\"", message),
            GitOperation::CreateTag { tag } => format!("git tag {}", tag),
            GitOperation::PushTag { remote, tag } => format!("git push {} {}", remote, tag),
            GitOperation::PushBranch { remote, branch } => {
                format!("git push {} {}", remote, branch)
            }
            GitOperation::PushAll { remote } => format!("git push --all {}", remote),
            GitOperation::PushTags { remote } => format!("git push --tags {}", remote),
            GitOperation::Pull { remote, branch } => format!("git pull {} {}", remote, branch),
            GitOperation::Checkout { ref_name } => format!("git checkout {}", ref_name),
            GitOperation::BranchDelete { branch } => format!("git branch -d {}", branch),
            GitOperation::BranchRename { old, new } => {
                format!("git branch -m {} {}", old, new)
            }
            GitOperation::RemoteDelete { remote, branch } => {
                format!("git push {} --delete {}", remote, branch)
            }
            GitOperation::RemoteRename { old, new } => {
                format!("git remote rename {} {}", old, new)
            }
            GitOperation::RemotePrune { remote } => format!("git remote prune {}", remote),
            GitOperation::SetUpstream { remote, branch } => {
                format!("git branch --set-upstream-to {}/{}", remote, branch)
            }
            GitOperation::Gc => "git gc --aggressive".to_string(),
            GitOperation::Custom { description, .. } => description.clone(),
        }
    }
}

pub struct ExecutionPlan {
    pub operations: Vec<GitOperation>,
    pub dry_run: bool,
}

impl ExecutionPlan {
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            dry_run: false,
        }
    }

    pub fn dry_run(mut self, value: bool) -> Self {
        self.dry_run = value;
        self
    }

    pub fn add(&mut self, op: GitOperation) {
        self.operations.push(op);
    }

    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    pub fn display(&self) {
        if self.operations.is_empty() {
            Output::skip("无操作");
            return;
        }
        for op in &self.operations {
            Output::message(&op.description());
        }
    }

    pub fn execute(&self) -> anyhow::Result<()> {
        if self.dry_run {
            Output::dry_run_header("将要执行的操作:");
            self.display();
            return Ok(());
        }

        let runner = GitCommandRunner::new();
        for op in &self.operations {
            Output::cmd(&op.description());
            match op {
                GitOperation::Init { dir } => {
                    runner.execute_with_success(&["init"], Some(dir))?
                }
                GitOperation::Clone { url, dir } => {
                    runner.execute_streaming(&["clone", url], dir)?
                }
                GitOperation::Add { path } => {
                    runner.execute_with_success(&["add", path], None)?
                }
                GitOperation::Commit { message } => {
                    runner.execute_with_success(&["commit", "-m", message], None)?
                }
                GitOperation::CreateTag { tag } => {
                    runner.execute_with_success(&["tag", tag], None)?
                }
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
                GitOperation::RemoteRename { old, new } => runner.execute_with_success(
                    &["remote", "rename", old, new],
                    Some(Path::new(".")),
                )?,
                GitOperation::RemotePrune { remote } => runner.execute_with_success(
                    &["remote", "prune", remote],
                    Some(Path::new(".")),
                )?,
                GitOperation::SetUpstream { remote, branch } => runner.execute_with_success(
                    &["branch", "--set-upstream-to", &format!("{}/{}", remote, branch)],
                    Some(Path::new(".")),
                )?,
                GitOperation::Gc => {
                    runner.execute_with_success(&["gc", "--aggressive"], None)?
                }
                GitOperation::Custom { args, .. } => {
                    let args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                    runner.execute(&args, Some(Path::new(".")))?;
                }
            }
        }
        Ok(())
    }
}

impl Default for ExecutionPlan {
    fn default() -> Self {
        Self::new()
    }
}
