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

struct GitInvocation {
    args: Vec<String>,
    working_dir: PathBuf,
    streaming: bool,
}

impl GitInvocation {
    fn local(working_dir: PathBuf, args: &[&str]) -> Self {
        Self {
            args: args.iter().map(|s| s.to_string()).collect(),
            working_dir,
            streaming: false,
        }
    }

    fn streaming(working_dir: PathBuf, args: &[&str]) -> Self {
        Self {
            args: args.iter().map(|s| s.to_string()).collect(),
            working_dir,
            streaming: true,
        }
    }

    fn display_command(&self) -> String {
        let mut parts: Vec<String> = Vec::with_capacity(self.args.len() + 1);
        parts.push("git".to_string());
        let quotes_message = self.args.first().map(String::as_str) == Some("commit");
        let mut iter = self.args.iter().peekable();
        while let Some(arg) = iter.next() {
            if quotes_message
                && arg == "-m"
                && let Some(message) = iter.next()
            {
                parts.push("-m".to_string());
                parts.push(format!("\"{}\"", message));
            } else {
                parts.push(arg.clone());
            }
        }
        parts.join(" ")
    }

    fn execute(&self, runner: &GitCommandRunner) -> crate::error::Result<()> {
        let args: Vec<&str> = self.args.iter().map(String::as_str).collect();
        if self.streaming {
            runner.run_streaming(&args, self.working_dir.as_path())?;
        } else {
            runner.run_local(&args, Some(self.working_dir.as_path()))?;
        }
        Ok(())
    }
}

impl GitOperation {
    fn invocation(&self) -> GitInvocation {
        match self {
            GitOperation::Init { working_dir } => {
                GitInvocation::local(working_dir.clone(), &["init"])
            }
            GitOperation::Clone {
                url,
                target_dir,
                working_dir,
            } => GitInvocation::streaming(
                working_dir.clone(),
                &["clone", url, target_dir.to_str().unwrap_or(".")],
            ),
            GitOperation::Add { path, working_dir } => {
                GitInvocation::local(working_dir.clone(), &["add", path])
            }
            GitOperation::Commit {
                message,
                working_dir,
            } => GitInvocation::local(working_dir.clone(), &["commit", "-m", message]),
            GitOperation::CreateTag { tag, working_dir } => {
                GitInvocation::local(working_dir.clone(), &["tag", tag])
            }
            GitOperation::PushTag {
                remote,
                tag,
                working_dir,
            } => GitInvocation::streaming(working_dir.clone(), &["push", remote, tag]),
            GitOperation::PushBranch {
                remote,
                branch,
                working_dir,
            } => GitInvocation::streaming(working_dir.clone(), &["push", remote, branch]),
            GitOperation::PushAll {
                remote,
                working_dir,
            } => GitInvocation::streaming(working_dir.clone(), &["push", "--all", remote]),
            GitOperation::PushTags {
                remote,
                working_dir,
            } => GitInvocation::streaming(working_dir.clone(), &["push", "--tags", remote]),
            GitOperation::Pull {
                remote,
                branch,
                working_dir,
            } => GitInvocation::streaming(working_dir.clone(), &["pull", remote, branch]),
            GitOperation::PullDefault { working_dir } => {
                GitInvocation::streaming(working_dir.clone(), &["pull"])
            }
            GitOperation::Checkout {
                ref_name,
                working_dir,
            } => GitInvocation::streaming(working_dir.clone(), &["checkout", ref_name]),
            GitOperation::DeleteBranch {
                branch,
                working_dir,
            } => GitInvocation::local(working_dir.clone(), &["branch", "-d", branch]),
            GitOperation::RenameBranch {
                old,
                new,
                working_dir,
            } => GitInvocation::streaming(working_dir.clone(), &["branch", "-m", old, new]),
            GitOperation::DeleteRemoteBranch {
                remote,
                branch,
                working_dir,
            } => GitInvocation::streaming(
                working_dir.clone(),
                &["push", remote, "--delete", branch],
            ),
            GitOperation::RenameRemote {
                old,
                new,
                working_dir,
            } => GitInvocation::local(working_dir.clone(), &["remote", "rename", old, new]),
            GitOperation::PruneRemote {
                remote,
                working_dir,
            } => GitInvocation::local(working_dir.clone(), &["remote", "prune", remote]),
            GitOperation::SetUpstream {
                remote,
                branch,
                working_dir,
            } => {
                let upstream = format!("{}/{}", remote, branch);
                GitInvocation {
                    args: vec![
                        "branch".to_string(),
                        "--set-upstream-to".to_string(),
                        upstream,
                    ],
                    working_dir: working_dir.clone(),
                    streaming: false,
                }
            }
            GitOperation::Gc { working_dir } => {
                GitInvocation::streaming(working_dir.clone(), &["gc", "--aggressive"])
            }
        }
    }

    pub fn description(&self) -> String {
        let inv = self.invocation();
        format!("[{}] {}", inv.working_dir.display(), inv.display_command())
    }

    pub fn execute(&self, runner: &GitCommandRunner) -> crate::error::Result<()> {
        self.invocation().execute(runner)
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
