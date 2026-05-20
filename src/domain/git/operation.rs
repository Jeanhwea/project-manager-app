use super::GitCommandRunner;
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

    pub fn execute(&self, runner: &GitCommandRunner) -> crate::error::Result<()> {
        match self {
            GitOperation::Init { working_dir } => {
                runner.run_local(&["init"], Some(working_dir.as_path()))?;
            }
            GitOperation::Clone {
                url,
                target_dir,
                working_dir,
            } => {
                runner.run_streaming(
                    &["clone", url, target_dir.to_str().unwrap_or(".")],
                    working_dir.as_path(),
                )?;
            }
            GitOperation::Add { path, working_dir } => {
                runner.run_local(&["add", path], Some(working_dir.as_path()))?;
            }
            GitOperation::Commit {
                message,
                working_dir,
            } => {
                runner.run_local(&["commit", "-m", message], Some(working_dir.as_path()))?;
            }
            GitOperation::CreateTag { tag, working_dir } => {
                runner.run_local(&["tag", tag], Some(working_dir.as_path()))?;
            }
            GitOperation::PushTag {
                remote,
                tag,
                working_dir,
            } => {
                runner.run_streaming(&["push", remote, tag], working_dir.as_path())?;
            }
            GitOperation::PushBranch {
                remote,
                branch,
                working_dir,
            } => {
                runner.run_streaming(&["push", remote, branch], working_dir.as_path())?;
            }
            GitOperation::PushAll {
                remote,
                working_dir,
            } => {
                runner.run_streaming(&["push", "--all", remote], working_dir.as_path())?;
            }
            GitOperation::PushTags {
                remote,
                working_dir,
            } => {
                runner.run_streaming(&["push", "--tags", remote], working_dir.as_path())?;
            }
            GitOperation::Pull {
                remote,
                branch,
                working_dir,
            } => {
                runner.run_streaming(&["pull", remote, branch], working_dir.as_path())?;
            }
            GitOperation::PullDefault { working_dir } => {
                runner.run_streaming(&["pull"], working_dir.as_path())?;
            }
            GitOperation::Checkout {
                ref_name,
                working_dir,
            } => {
                runner.run_streaming(&["checkout", ref_name], working_dir.as_path())?;
            }
            GitOperation::DeleteBranch {
                branch,
                working_dir,
            } => {
                runner.run_local(&["branch", "-d", branch], Some(working_dir.as_path()))?;
            }
            GitOperation::RenameBranch {
                old,
                new,
                working_dir,
            } => {
                runner.run_streaming(&["branch", "-m", old, new], working_dir.as_path())?;
            }
            GitOperation::DeleteRemoteBranch {
                remote,
                branch,
                working_dir,
            } => {
                runner.run_streaming(
                    &["push", remote, "--delete", branch],
                    working_dir.as_path(),
                )?;
            }
            GitOperation::RenameRemote {
                old,
                new,
                working_dir,
            } => {
                runner.run_local(
                    &["remote", "rename", old, new],
                    Some(working_dir.as_path()),
                )?;
            }
            GitOperation::PruneRemote {
                remote,
                working_dir,
            } => {
                runner.run_local(&["remote", "prune", remote], Some(working_dir.as_path()))?;
            }
            GitOperation::SetUpstream {
                remote,
                branch,
                working_dir,
            } => {
                let upstream = format!("{}/{}", remote, branch);
                runner.run_local(
                    &["branch", "--set-upstream-to", &upstream],
                    Some(working_dir.as_path()),
                )?;
            }
            GitOperation::Gc { working_dir } => {
                runner.run_streaming(&["gc", "--aggressive"], working_dir.as_path())?;
            }
        }
        Ok(())
    }

    pub fn recovery_hint(&self, _executed_count: usize) -> Option<String> {
        match self {
            GitOperation::PushTag { remote, tag, .. } => Some(format!(
                "tag {} 已创建但未推送，请手动执行: git push {} {}",
                tag, remote, tag
            )),
            GitOperation::PushBranch { remote, branch, .. } => Some(format!(
                "commit 已创建但未推送，请手动执行: git push {} {}",
                remote, branch
            )),
            GitOperation::PushAll { remote, .. } => Some(format!(
                "commit 已创建但未推送，请手动执行: git push --all {}",
                remote
            )),
            GitOperation::PushTags { remote, .. } => Some(format!(
                "tag 已创建但未推送，请手动执行: git push --tags {}",
                remote
            )),
            _ => None,
        }
    }

    pub fn should_skip(&self) -> Option<String> {
        match self {
            GitOperation::Clone { target_dir, .. } if target_dir.exists() => {
                Some(format!("目录 {} 已存在，跳过克隆", target_dir.display()))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_run_descriptions_byte_equal_baseline() {
        let init = GitOperation::Init {
            working_dir: PathBuf::from("."),
        };
        assert_eq!(init.description(), "[.] git init");

        let add = GitOperation::Add {
            path: ".".to_string(),
            working_dir: PathBuf::from("."),
        };
        assert_eq!(add.description(), "[.] git add .");

        let commit = GitOperation::Commit {
            message: "snap-000001".to_string(),
            working_dir: PathBuf::from("."),
        };
        assert_eq!(commit.description(), "[.] git commit -m \"snap-000001\"");

        let push_tag = GitOperation::PushTag {
            remote: "origin".to_string(),
            tag: "v1.0.0".to_string(),
            working_dir: PathBuf::from("."),
        };
        assert_eq!(push_tag.description(), "[.] git push origin v1.0.0");

        let push_branch = GitOperation::PushBranch {
            remote: "origin".to_string(),
            branch: "master".to_string(),
            working_dir: PathBuf::from("."),
        };
        assert_eq!(push_branch.description(), "[.] git push origin master");

        let clone = GitOperation::Clone {
            url: "https://x".to_string(),
            target_dir: PathBuf::from("repo"),
            working_dir: PathBuf::from("."),
        };
        assert_eq!(clone.description(), "[.] git clone https://x repo");

        let gc = GitOperation::Gc {
            working_dir: PathBuf::from("."),
        };
        assert_eq!(gc.description(), "[.] git gc --aggressive");
    }
}
