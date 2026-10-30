//! git 状态的只读快照模型：remote、branch、tag 以及由它们聚合而成的 `GitContext`。
//! 这些结构由 domain 层解析 git 输出后填充，命令层据此做判断而不自行调用 git。

#[derive(Debug, Clone)]
pub struct Remote {
    pub name: String,
    pub url: String,
}

impl Remote {
    /// 从 remote url 里取出主机名，失败返回 None
    pub fn extract_host(&self) -> Option<String> {
        // SSH 形式 git@host:org/repo.git 无法被 url::Url 解析，必须单独按 ':' 切分
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
}

impl Branch {
    pub fn local_branches(branches: &[Branch]) -> Vec<&Branch> {
        branches.iter().filter(|b| !b.is_remote).collect()
    }

    pub fn is_current_local(&self) -> bool {
        self.is_current && !self.is_remote
    }

    /// 取 tracking branch 的 remote 名：`origin/main` -> `origin`
    pub fn upstream_remote(&self) -> Option<String> {
        self.tracking_branch
            .as_ref()
            .and_then(|tracking| tracking.split('/').next().map(String::from))
    }
}

#[derive(Debug, Clone)]
pub struct Tag {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct GitContext {
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

    pub fn first_remote_name(&self) -> Option<String> {
        self.remotes.first().map(|r| r.name.clone())
    }

    pub fn current_branch_upstream_remote(&self) -> Option<String> {
        self.branches
            .iter()
            .find(|b| b.is_current_local())
            .and_then(Branch::upstream_remote)
    }

    /// 当前分支的上游 remote；若该 remote 在本仓库中并不存在则返回 None，避免拿到悬空的名字
    pub fn preferred_remote(&self) -> Option<String> {
        self.current_branch_upstream_remote()
            .filter(|name| self.has_remote(name))
    }

    // remote 分支在 branches 里的 name 形如 origin/main，因此这里拼接后再比较
    pub fn has_remote_branch(&self, remote: &str, branch: &str) -> bool {
        let remote_branch = format!("{}/{}", remote, branch);
        self.branches
            .iter()
            .any(|b| b.is_remote && b.name == remote_branch)
    }
}
