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

impl Branch {
    pub fn local_branches(branches: &[Branch]) -> Vec<&Branch> {
        branches.iter().filter(|b| !b.is_remote).collect()
    }

    pub fn remote_branches(branches: &[Branch]) -> Vec<&Branch> {
        branches.iter().filter(|b| b.is_remote).collect()
    }
}
