use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum GitOperation {
    Init {
        working_dir: PathBuf,
    },
    Clone {
        url: String,
        target_dir: PathBuf,
        working_dir: PathBuf,
    },
    Add {
        path: String,
        working_dir: PathBuf,
    },
    Commit {
        message: String,
        working_dir: PathBuf,
    },
    CreateTag {
        tag: String,
        working_dir: PathBuf,
    },
    PushTag {
        remote: String,
        tag: String,
        working_dir: PathBuf,
    },
    PushBranch {
        remote: String,
        branch: String,
        working_dir: PathBuf,
    },
    PushAll {
        remote: String,
        working_dir: PathBuf,
    },
    PushTags {
        remote: String,
        working_dir: PathBuf,
    },
    Pull {
        remote: String,
        branch: String,
        working_dir: PathBuf,
    },
    Checkout {
        ref_name: String,
        working_dir: PathBuf,
    },
    DeleteBranch {
        branch: String,
        working_dir: PathBuf,
    },
    RenameBranch {
        old: String,
        new: String,
        working_dir: PathBuf,
    },
    DeleteRemoteBranch {
        remote: String,
        branch: String,
        working_dir: PathBuf,
    },
    RenameRemote {
        old: String,
        new: String,
        working_dir: PathBuf,
    },
    PruneRemote {
        remote: String,
        working_dir: PathBuf,
    },
    SetUpstream {
        remote: String,
        branch: String,
        working_dir: PathBuf,
    },
    Gc {
        working_dir: PathBuf,
    },
}

impl GitOperation {
    fn format_working_dir(path: &PathBuf) -> String {
        if path == Path::new(".") {
            String::new()
        } else {
            format!(" in {}", path.display())
        }
    }

    pub fn description(&self) -> String {
        match self {
            GitOperation::Init { working_dir } => {
                format!("git init{}", Self::format_working_dir(working_dir))
            }
            GitOperation::Clone {
                url,
                target_dir,
                working_dir,
            } => {
                format!(
                    "git clone {} {}{}",
                    url,
                    target_dir.display(),
                    Self::format_working_dir(working_dir)
                )
            }
            GitOperation::Add { path, working_dir } => {
                format!("git add {}{}", path, Self::format_working_dir(working_dir))
            }
            GitOperation::Commit {
                message,
                working_dir,
            } => format!(
                "git commit -m \"{}\"{}",
                message,
                Self::format_working_dir(working_dir)
            ),
            GitOperation::CreateTag { tag, working_dir } => {
                format!("git tag {}{}", tag, Self::format_working_dir(working_dir))
            }
            GitOperation::PushTag {
                remote,
                tag,
                working_dir,
            } => format!(
                "git push {} {}{}",
                remote,
                tag,
                Self::format_working_dir(working_dir)
            ),
            GitOperation::PushBranch {
                remote,
                branch,
                working_dir,
            } => {
                format!(
                    "git push {} {}{}",
                    remote,
                    branch,
                    Self::format_working_dir(working_dir)
                )
            }
            GitOperation::PushAll {
                remote,
                working_dir,
            } => format!(
                "git push --all {}{}",
                remote,
                Self::format_working_dir(working_dir)
            ),
            GitOperation::PushTags {
                remote,
                working_dir,
            } => format!(
                "git push --tags {}{}",
                remote,
                Self::format_working_dir(working_dir)
            ),
            GitOperation::Pull {
                remote,
                branch,
                working_dir,
            } => format!(
                "git pull {} {}{}",
                remote,
                branch,
                Self::format_working_dir(working_dir)
            ),
            GitOperation::Checkout {
                ref_name,
                working_dir,
            } => format!(
                "git checkout {}{}",
                ref_name,
                Self::format_working_dir(working_dir)
            ),
            GitOperation::DeleteBranch {
                branch,
                working_dir,
            } => format!(
                "git branch -d {}{}",
                branch,
                Self::format_working_dir(working_dir)
            ),
            GitOperation::RenameBranch {
                old,
                new,
                working_dir,
            } => {
                format!(
                    "git branch -m {} {}{}",
                    old,
                    new,
                    Self::format_working_dir(working_dir)
                )
            }
            GitOperation::DeleteRemoteBranch {
                remote,
                branch,
                working_dir,
            } => {
                format!(
                    "git push {} --delete {}{}",
                    remote,
                    branch,
                    Self::format_working_dir(working_dir)
                )
            }
            GitOperation::RenameRemote {
                old,
                new,
                working_dir,
            } => {
                format!(
                    "git remote rename {} {}{}",
                    old,
                    new,
                    Self::format_working_dir(working_dir)
                )
            }
            GitOperation::PruneRemote {
                remote,
                working_dir,
            } => format!(
                "git remote prune {}{}",
                remote,
                Self::format_working_dir(working_dir)
            ),
            GitOperation::SetUpstream {
                remote,
                branch,
                working_dir,
            } => {
                format!(
                    "git branch --set-upstream-to {}/{}{}",
                    remote,
                    branch,
                    Self::format_working_dir(working_dir)
                )
            }
            GitOperation::Gc { working_dir } => {
                format!(
                    "git gc --aggressive{}",
                    Self::format_working_dir(working_dir)
                )
            }
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
}

impl ExecutionPlan {
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            dry_run: false,
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
