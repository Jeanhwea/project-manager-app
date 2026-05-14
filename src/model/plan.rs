use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct GitOperationContext {
    pub working_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub enum GitOperation {
    Init,
    Clone {
        url: String,
        target_dir: PathBuf,
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
    DeleteBranch {
        branch: String,
    },
    RenameBranch {
        old: String,
        new: String,
    },
    DeleteRemoteBranch {
        remote: String,
        branch: String,
    },
    RenameRemote {
        old: String,
        new: String,
    },
    PruneRemote {
        remote: String,
    },
    SetUpstream {
        remote: String,
        branch: String,
    },
    Gc,
}

impl GitOperation {
    pub fn description(&self) -> String {
        match self {
            GitOperation::Init => "[working_dir] git init".to_string(),
            GitOperation::Clone { url, target_dir } => {
                format!("[working_dir] git clone {} {}", url, target_dir.display())
            }
            GitOperation::Add { path } => "[working_dir] git add {}".to_string(),
            GitOperation::Commit { message } => "[working_dir] git commit -m \"{}\"".to_string(),
            GitOperation::CreateTag { tag } => "[working_dir] git tag {}".to_string(),
            GitOperation::PushTag { remote, tag } => "[working_dir] git push {} {}".to_string(),
            GitOperation::PushBranch { remote, branch } => {
                "[working_dir] git push {} {}".to_string()
            }
            GitOperation::PushAll { remote } => "[working_dir] git push --all {}".to_string(),
            GitOperation::PushTags { remote } => "[working_dir] git push --tags {}".to_string(),
            GitOperation::Pull { remote, branch } => "[working_dir] git pull {} {}".to_string(),
            GitOperation::Checkout { ref_name } => "[working_dir] git checkout {}".to_string(),
            GitOperation::DeleteBranch { branch } => "[working_dir] git branch -d {}".to_string(),
            GitOperation::RenameBranch { old, new } => "[working_dir] git branch -m {} {}".to_string(),
            GitOperation::DeleteRemoteBranch { remote, branch } => {
                "[working_dir] git push {} --delete {}".to_string()
            }
            GitOperation::RenameRemote { old, new } => "[working_dir] git remote rename {} {}".to_string(),
            GitOperation::PruneRemote { remote } => "[working_dir] git remote prune {}".to_string(),
            GitOperation::SetUpstream { remote, branch } => {
                "[working_dir] git branch --set-upstream-to {}/{}".to_string()
            }
            GitOperation::Gc => "[working_dir] git gc --aggressive".to_string(),
        }
    }

    pub fn description_with_context(&self, ctx: &GitOperationContext) -> String {
        let base = self.description();
        base.replace("[working_dir]", &ctx.working_dir.display().to_string())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn git_description_omits_current_working_dir() {
        let op = GitOperation::Add {
            path: "src/main.rs".to_string(),
        };

        assert_eq!(op.description(), "[working_dir] git add src/main.rs");
    }

    #[test]
    fn git_description_includes_non_current_working_dir() {
        let op = GitOperation::Add {
            path: "src/main.rs".to_string(),
        };

        assert_eq!(op.description(), "[working_dir] git add src/main.rs");
    }

    #[test]
    fn git_description_with_context() {
        let op = GitOperation::Add {
            path: "src/main.rs".to_string(),
        };
        let ctx = GitOperationContext {
            working_dir: PathBuf::from("repo"),
        };

        assert_eq!(op.description_with_context(&ctx), "[repo] git add src/main.rs");
    }

    #[test]
    fn message_diff_description_includes_file_and_line_numbers() {
        let diff = MessageOperation::Diff {
            file: "pyproject.toml".to_string(),
            line_num: 3,
            old_content: "version = \"1.4.10\"".to_string(),
            new_content: "version = \"1.5.0\"".to_string(),
        };

        let expected = "pyproject.toml L3 -:  version = \"1.4.10\"\npyproject.toml L3 +:  version = \"1.5.0\"";
        assert_eq!(diff.description(), expected);
    }
}
