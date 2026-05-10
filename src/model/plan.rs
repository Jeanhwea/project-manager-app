use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum GitOperation {
    Init {
        dir: PathBuf,
    },
    Clone {
        url: String,
        dir: PathBuf,
    },
    Add {
        path: String,
    },
    Commit {
        message: String,
    },
    CreateTag {
        tag: String,
    },
    PushTag {
        remote: String,
        tag: String,
    },
    PushBranch {
        remote: String,
        branch: String,
    },
    PushAll {
        remote: String,
    },
    PushTags {
        remote: String,
    },
    Pull {
        remote: String,
        branch: String,
    },
    Checkout {
        ref_name: String,
    },
    BranchDelete {
        branch: String,
    },
    BranchRename {
        old: String,
        new: String,
    },
    RemoteDelete {
        remote: String,
        branch: String,
    },
    RemoteRename {
        old: String,
        new: String,
    },
    RemotePrune {
        remote: String,
    },
    SetUpstream {
        remote: String,
        branch: String,
    },
    Gc,
    Custom {
        args: Vec<String>,
        description: String,
    },
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
}

impl Default for ExecutionPlan {
    fn default() -> Self {
        Self::new()
    }
}
