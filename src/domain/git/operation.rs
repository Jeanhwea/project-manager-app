use std::path::PathBuf;

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
    PullDefault {
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
    pub fn description(&self) -> String {
        match self {
            GitOperation::Init { working_dir } => {
                format!("[{}] git init", working_dir.display())
            }
            GitOperation::Clone {
                url,
                target_dir,
                working_dir,
            } => {
                format!(
                    "[{}] git clone {} {}",
                    working_dir.display(),
                    url,
                    target_dir.display()
                )
            }
            GitOperation::Add { path, working_dir } => {
                format!("[{}] git add {}", working_dir.display(), path)
            }
            GitOperation::Commit {
                message,
                working_dir,
            } => format!("[{}] git commit -m \"{}\"", working_dir.display(), message),
            GitOperation::CreateTag { tag, working_dir } => {
                format!("[{}] git tag {}", working_dir.display(), tag)
            }
            GitOperation::PushTag {
                remote,
                tag,
                working_dir,
            } => format!("[{}] git push {} {}", working_dir.display(), remote, tag),
            GitOperation::PushBranch {
                remote,
                branch,
                working_dir,
            } => {
                format!("[{}] git push {} {}", working_dir.display(), remote, branch)
            }
            GitOperation::PushAll {
                remote,
                working_dir,
            } => format!("[{}] git push --all {}", working_dir.display(), remote),
            GitOperation::PushTags {
                remote,
                working_dir,
            } => format!("[{}] git push --tags {}", working_dir.display(), remote),
            GitOperation::Pull {
                remote,
                branch,
                working_dir,
            } => format!("[{}] git pull {} {}", working_dir.display(), remote, branch),
            GitOperation::PullDefault { working_dir } => {
                format!("[{}] git pull", working_dir.display())
            }
            GitOperation::Checkout {
                ref_name,
                working_dir,
            } => format!("[{}] git checkout {}", working_dir.display(), ref_name),
            GitOperation::DeleteBranch {
                branch,
                working_dir,
            } => format!("[{}] git branch -d {}", working_dir.display(), branch),
            GitOperation::RenameBranch {
                old,
                new,
                working_dir,
            } => {
                format!("[{}] git branch -m {} {}", working_dir.display(), old, new)
            }
            GitOperation::DeleteRemoteBranch {
                remote,
                branch,
                working_dir,
            } => {
                format!(
                    "[{}] git push {} --delete {}",
                    working_dir.display(),
                    remote,
                    branch
                )
            }
            GitOperation::RenameRemote {
                old,
                new,
                working_dir,
            } => {
                format!(
                    "[{}] git remote rename {} {}",
                    working_dir.display(),
                    old,
                    new
                )
            }
            GitOperation::PruneRemote {
                remote,
                working_dir,
            } => format!("[{}] git remote prune {}", working_dir.display(), remote),
            GitOperation::SetUpstream {
                remote,
                branch,
                working_dir,
            } => {
                format!(
                    "[{}] git branch --set-upstream-to {}/{}",
                    working_dir.display(),
                    remote,
                    branch
                )
            }
            GitOperation::Gc { working_dir } => {
                format!("[{}] git gc --aggressive", working_dir.display())
            }
        }
    }
}
