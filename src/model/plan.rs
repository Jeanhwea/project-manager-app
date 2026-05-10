use std::path::PathBuf;

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
        }
    }
}

#[derive(Debug, Clone)]
pub enum ShellOperation {
    Run {
        program: String,
        args: Vec<String>,
        dir: Option<PathBuf>,
        description: String,
    },
}

impl ShellOperation {
    pub fn description(&self) -> String {
        match self {
            ShellOperation::Run { description, .. } => description.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum EditOperation {
    WriteFile {
        path: String,
        content: String,
        description: String,
    },
}

impl EditOperation {
    pub fn description(&self) -> String {
        match self {
            EditOperation::WriteFile { description, .. } => description.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Operation {
    Git(GitOperation),
    Shell(ShellOperation),
    Edit(EditOperation),
}

impl Operation {
    pub fn description(&self) -> String {
        match self {
            Operation::Git(op) => op.description(),
            Operation::Shell(op) => op.description(),
            Operation::Edit(op) => op.description(),
        }
    }
}

impl From<GitOperation> for Operation {
    fn from(op: GitOperation) -> Self {
        Operation::Git(op)
    }
}

impl From<ShellOperation> for Operation {
    fn from(op: ShellOperation) -> Self {
        Operation::Shell(op)
    }
}

impl From<EditOperation> for Operation {
    fn from(op: EditOperation) -> Self {
        Operation::Edit(op)
    }
}

pub struct ExecutionPlan {
    pub operations: Vec<Operation>,
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

    pub fn add(&mut self, op: impl Into<Operation>) {
        self.operations.push(op.into());
    }

    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }
}

impl Default for ExecutionPlan {
    fn default() -> Self {
        Self::new()
    }
}
