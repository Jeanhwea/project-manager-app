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
    DeleteBranch { branch: String },
    RenameBranch { old: String, new: String },
    DeleteRemoteBranch { remote: String, branch: String },
    RenameRemote { old: String, new: String },
    PruneRemote { remote: String },
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
            GitOperation::DeleteBranch { branch } => format!("git branch -d {}", branch),
            GitOperation::RenameBranch { old, new } => {
                format!("git branch -m {} {}", old, new)
            }
            GitOperation::DeleteRemoteBranch { remote, branch } => {
                format!("git push {} --delete {}", remote, branch)
            }
            GitOperation::RenameRemote { old, new } => {
                format!("git remote rename {} {}", old, new)
            }
            GitOperation::PruneRemote { remote } => format!("git remote prune {}", remote),
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
    CopyDir {
        source: String,
        target: String,
        description: String,
    },
}

impl EditOperation {
    pub fn description(&self) -> String {
        match self {
            EditOperation::WriteFile { description, .. } => description.clone(),
            EditOperation::CopyDir { description, .. } => description.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SelfUpdateOperation {
    DownloadAndInstall {
        api_url: String,
        browser_url: String,
        asset_name: String,
        current_version: String,
        target_version: String,
    },
}

impl SelfUpdateOperation {
    pub fn description(&self) -> String {
        match self {
            SelfUpdateOperation::DownloadAndInstall {
                asset_name,
                current_version,
                target_version,
                ..
            } => {
                format!(
                    "download {} and update v{} -> v{}",
                    asset_name, current_version, target_version
                )
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum MessageOperation {
    Header {
        title: String,
    },
    Section {
        title: String,
    },
    Item {
        label: String,
        value: String,
    },
    Detail {
        label: String,
        value: String,
    },
    Diff {
        file: String,
        line_num: usize,
        old_content: String,
        new_content: String,
    },
    Success {
        msg: String,
    },
    Warning {
        msg: String,
    },
    Skip {
        msg: String,
    },
    Blank,
}

impl MessageOperation {
    pub fn description(&self) -> String {
        match self {
            MessageOperation::Header { title } => title.clone(),
            MessageOperation::Section { title } => title.clone(),
            MessageOperation::Item { label, value } => format!("{}: {}", label, value),
            MessageOperation::Detail { label, value } => format!("  {}: {}", label, value),
            MessageOperation::Diff {
                file,
                line_num,
                old_content,
                new_content,
            } => format!(
                "{} L{} -:  {}\n{} L{} +:  {}",
                file, line_num, old_content, file, line_num, new_content
            ),
            MessageOperation::Success { msg } => format!("OK> {}", msg),
            MessageOperation::Warning { msg } => format!("WARN {}", msg),
            MessageOperation::Skip { msg } => format!("SKIP {}", msg),
            MessageOperation::Blank => String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Operation {
    Git(GitOperation),
    Shell(ShellOperation),
    Edit(EditOperation),
    SelfUpdate(SelfUpdateOperation),
    Message(MessageOperation),
}

impl Operation {
    pub fn description(&self) -> String {
        match self {
            Operation::Git(op) => op.description(),
            Operation::Shell(op) => op.description(),
            Operation::Edit(op) => op.description(),
            Operation::SelfUpdate(op) => op.description(),
            Operation::Message(op) => op.description(),
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

impl From<SelfUpdateOperation> for Operation {
    fn from(op: SelfUpdateOperation) -> Self {
        Operation::SelfUpdate(op)
    }
}

impl From<MessageOperation> for Operation {
    fn from(op: MessageOperation) -> Self {
        Operation::Message(op)
    }
}

pub struct ExecutionPlan {
    pub operations: Vec<Operation>,
    pub dry_run: bool,
    pub repo_path: Option<PathBuf>,
}

impl ExecutionPlan {
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            dry_run: false,
            repo_path: None,
        }
    }

    pub fn with_dry_run(mut self, value: bool) -> Self {
        self.dry_run = value;
        self
    }

    pub fn add(&mut self, op: impl Into<Operation>) {
        self.operations.push(op.into());
    }
}

impl Default for ExecutionPlan {
    fn default() -> Self {
        Self::new()
    }
}
