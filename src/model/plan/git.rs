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
                format!("git init in {}", working_dir.display())
            }
            GitOperation::Clone {
                url,
                target_dir,
                working_dir,
            } => {
                format!(
                    "git clone {} {} in {}",
                    url,
                    target_dir.display(),
                    working_dir.display()
                )
            }
            GitOperation::Add { path, working_dir } => {
                format!("git add {} in {}", path, working_dir.display())
            }
            GitOperation::Commit {
                message,
                working_dir,
            } => format!("git commit -m \"{}\" in {}", message, working_dir.display()),
            GitOperation::CreateTag { tag, working_dir } => {
                format!("git tag {} in {}", tag, working_dir.display())
            }
            GitOperation::PushTag {
                remote,
                tag,
                working_dir,
            } => format!("git push {} {} in {}", remote, tag, working_dir.display()),
            GitOperation::PushBranch {
                remote,
                branch,
                working_dir,
            } => {
                format!(
                    "git push {} {} in {}",
                    remote,
                    branch,
                    working_dir.display()
                )
            }
            GitOperation::PushAll {
                remote,
                working_dir,
            } => format!("git push --all {} in {}", remote, working_dir.display()),
            GitOperation::PushTags {
                remote,
                working_dir,
            } => format!("git push --tags {} in {}", remote, working_dir.display()),
            GitOperation::Pull {
                remote,
                branch,
                working_dir,
            } => format!(
                "git pull {} {} in {}",
                remote,
                branch,
                working_dir.display()
            ),
            GitOperation::Checkout {
                ref_name,
                working_dir,
            } => format!("git checkout {} in {}", ref_name, working_dir.display()),
            GitOperation::DeleteBranch {
                branch,
                working_dir,
            } => format!("git branch -d {} in {}", branch, working_dir.display()),
            GitOperation::RenameBranch {
                old,
                new,
                working_dir,
            } => {
                format!("git branch -m {} {} in {}", old, new, working_dir.display())
            }
            GitOperation::DeleteRemoteBranch {
                remote,
                branch,
                working_dir,
            } => {
                format!(
                    "git push {} --delete {} in {}",
                    remote,
                    branch,
                    working_dir.display()
                )
            }
            GitOperation::RenameRemote {
                old,
                new,
                working_dir,
            } => {
                format!(
                    "git remote rename {} {} in {}",
                    old,
                    new,
                    working_dir.display()
                )
            }
            GitOperation::PruneRemote {
                remote,
                working_dir,
            } => format!("git remote prune {} in {}", remote, working_dir.display()),
            GitOperation::SetUpstream {
                remote,
                branch,
                working_dir,
            } => {
                format!(
                    "git branch --set-upstream-to {}/{} in {}",
                    remote,
                    branch,
                    working_dir.display()
                )
            }
            GitOperation::Gc { working_dir } => {
                format!("git gc --aggressive in {}", working_dir.display())
            }
        }
    }
}
