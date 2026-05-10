use super::Result;
use super::command::GitCommandRunner;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Remote {
    pub name: String,
    pub url: String,
    pub fetch_url: Option<String>,
}

impl Remote {
    pub fn extract_host(&self) -> Option<String> {
        if self.url.starts_with("git@") {
            self.url
                .split(':')
                .next()
                .and_then(|s| s.strip_prefix("git@"))
                .map(String::from)
        } else if let Ok(url) = url::Url::parse(&self.url) {
            url.host_str().map(String::from)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct Branch {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
    pub tracking_branch: Option<String>,
    pub ahead_behind: Option<(usize, usize)>,
}

#[derive(Debug, Clone)]
pub struct Tag {
    pub name: String,
    pub commit: String,
    pub is_annotated: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Repository {
    pub path: std::path::PathBuf,
    pub root: std::path::PathBuf,
    pub current_branch: Option<String>,
    pub remotes: Vec<Remote>,
    pub branches: Vec<Branch>,
    pub tags: Vec<Tag>,
    pub has_uncommitted_changes: bool,
}

impl Repository {
    pub fn load(path: &Path) -> Result<Self> {
        let runner = GitCommandRunner::new();

        let root = runner.execute(&["rev-parse", "--show-toplevel"], Some(path))?;
        let root = std::path::PathBuf::from(root);

        let current_branch = runner
            .execute(&["branch", "--show-current"], Some(&root))
            .ok()
            .filter(|s| !s.is_empty());

        let remotes = Self::load_remotes(&runner, &root)?;
        let branches = Self::load_branches(&runner, &root, &current_branch)?;
        let tags = Self::load_tags(&runner, &root)?;
        let has_uncommitted_changes = runner.has_uncommitted_changes(&root)?;

        Ok(Self {
            path: path.to_path_buf(),
            root,
            current_branch,
            remotes,
            branches,
            tags,
            has_uncommitted_changes,
        })
    }

    fn load_remotes(runner: &GitCommandRunner, root: &Path) -> Result<Vec<Remote>> {
        let output = runner.execute(&["remote"], Some(root))?;
        let names: Vec<&str> = output
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();

        let mut remotes = Vec::new();
        for name in names {
            let url = runner
                .execute(&["remote", "get-url", name], Some(root))
                .ok();
            let fetch_url = runner
                .execute(&["remote", "get-url", "--push", name], Some(root))
                .ok();
            if let Some(url) = url {
                let fetch_url = fetch_url.filter(|u| *u != url);
                remotes.push(Remote {
                    name: name.to_string(),
                    url,
                    fetch_url,
                });
            }
        }
        Ok(remotes)
    }

    fn load_branches(
        runner: &GitCommandRunner,
        root: &Path,
        _current_branch: &Option<String>,
    ) -> Result<Vec<Branch>> {
        let output = runner.execute(&["branch", "-vv", "--all"], Some(root))?;
        let mut branches = Vec::new();

        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let is_current = line.starts_with("* ");
            let line = line.trim_start_matches("* ").trim_start_matches("  ");

            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            let name = parts.first().unwrap_or(&line).to_string();
            let name = name.trim_start_matches("remotes/").to_string();

            let is_remote = line.contains("remotes/");

            let tracking_branch = Self::extract_tracking_branch(parts.get(1).unwrap_or(&""));
            let ahead_behind = Self::extract_ahead_behind(parts.get(1).unwrap_or(&""));

            branches.push(Branch {
                name,
                is_current: is_current && !is_remote,
                is_remote,
                tracking_branch,
                ahead_behind,
            });
        }

        Ok(branches)
    }

    fn extract_tracking_branch(info: &str) -> Option<String> {
        if let Some(start) = info.find('[')
            && let Some(end) = info.find(']')
        {
            let inner = &info[start + 1..end];
            if inner.contains(":") {
                Some(inner.to_string())
            } else {
                None
            }
        } else {
            None
        }
    }

    fn extract_ahead_behind(info: &str) -> Option<(usize, usize)> {
        if let Some(start) = info.find('[')
            && let Some(end) = info.find(']')
        {
            let inner = &info[start + 1..end];
            if let Some(ahead) = inner.strip_prefix("ahead ") {
                if let Some(space) = ahead.find(' ')
                    && let Some(behind) = ahead[space + 1..].strip_prefix("behind ")
                    && let (Ok(a), Ok(b)) = (ahead[..space].parse(), behind.parse())
                {
                    return Some((a, b));
                }
            } else if let Some(behind) = inner.strip_prefix("behind ")
                && let Ok(b) = behind.parse::<usize>()
            {
                return Some((0, b));
            } else if let Some(ahead) = inner.strip_prefix("ahead ")
                && let Ok(a) = ahead.parse::<usize>()
            {
                return Some((a, 0));
            }
        }
        None
    }

    fn load_tags(runner: &GitCommandRunner, root: &Path) -> Result<Vec<Tag>> {
        let output = runner.execute(
            &[
                "for-each-ref",
                "--format=%(refname:short) %(objectname:short) %(objecttype)",
                "refs/tags",
            ],
            Some(root),
        )?;

        let mut tags = Vec::new();
        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            if parts.len() >= 2 {
                tags.push(Tag {
                    name: parts[0].to_string(),
                    commit: parts[1].to_string(),
                    is_annotated: parts.get(2).map(|t| t == &"tag").unwrap_or(false),
                    message: None,
                });
            }
        }

        Ok(tags)
    }

    pub fn local_branches(&self) -> Vec<&Branch> {
        self.branches.iter().filter(|b| !b.is_remote).collect()
    }

    pub fn remote_branches(&self) -> Vec<&Branch> {
        self.branches.iter().filter(|b| b.is_remote).collect()
    }

    pub fn has_remote(&self, name: &str) -> bool {
        self.remotes.iter().any(|r| r.name == name)
    }

    pub fn has_branch(&self, name: &str) -> bool {
        self.branches.iter().any(|b| b.name == name && !b.is_remote)
    }

    pub fn has_tag(&self, name: &str) -> bool {
        self.tags.iter().any(|t| t.name == name)
    }
}
