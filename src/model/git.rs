use std::path::PathBuf;

#[derive(Debug, Clone)]
#[allow(dead_code)]
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
#[allow(dead_code)]
pub struct Branch {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
    pub tracking_branch: Option<String>,
    pub ahead_behind: Option<(usize, usize)>,
}

impl Branch {
    pub fn local_branches(branches: &[Branch]) -> Vec<&Branch> {
        branches.iter().filter(|b| !b.is_remote).collect()
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Tag {
    pub name: String,
    pub commit: String,
    pub is_annotated: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct GitContext {
    pub root: PathBuf,
    pub current_branch: String,
    pub remotes: Vec<Remote>,
    pub branches: Vec<Branch>,
    pub tags: Vec<Tag>,
    pub has_uncommitted_changes: bool,
}

impl GitContext {
    pub fn remote_names(&self) -> Vec<&str> {
        self.remotes.iter().map(|r| r.name.as_str()).collect()
    }

    pub fn has_remote(&self, name: &str) -> bool {
        self.remotes.iter().any(|r| r.name == name)
    }

    pub fn local_branches(&self) -> Vec<&Branch> {
        Branch::local_branches(&self.branches)
    }
}
