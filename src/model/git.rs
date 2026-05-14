#[derive(Debug, Clone)]
pub struct Remote {
    pub name: String,
    pub url: String,
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
}

impl Branch {
    pub fn local_branches(branches: &[Branch]) -> Vec<&Branch> {
        branches.iter().filter(|b| !b.is_remote).collect()
    }

    pub fn is_current_local(&self) -> bool {
        self.is_current && !self.is_remote
    }

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

    pub fn preferred_remote(&self) -> Option<String> {
        self.current_branch_upstream_remote()
            .filter(|name| self.has_remote(name))
    }

    pub fn has_remote_branch(&self, remote: &str, branch: &str) -> bool {
        let remote_branch = format!("{}/{}", remote, branch);
        self.branches
            .iter()
            .any(|b| b.is_remote && b.name == remote_branch)
    }
}

use clap::ValueEnum;
use semver::Version as SemVersion;

#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum BumpType {
    #[value(alias = "ma")]
    Major,
    #[value(alias = "mi")]
    Minor,
    #[value(alias = "pa")]
    Patch,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub fn parse(s: &str) -> Result<Self, crate::error::AppError> {
        let ver = SemVersion::parse(s.trim()).map_err(|e| {
            crate::error::AppError::VersionFormatError(format!("Invalid version format: {}", e))
        })?;
        Ok(Version {
            major: ver.major as u32,
            minor: ver.minor as u32,
            patch: ver.patch as u32,
        })
    }

    pub fn from_tag(tag: &str) -> Option<Self> {
        let tag = tag.trim();
        let version_str = tag.strip_prefix('v').unwrap_or(tag);
        Self::parse(version_str).ok()
    }

    pub fn bump(&self, bump_type: &BumpType) -> Self {
        match bump_type {
            BumpType::Major => Version {
                major: self.major + 1,
                minor: 0,
                patch: 0,
            },
            BumpType::Minor => Version {
                major: self.major,
                minor: self.minor + 1,
                patch: 0,
            },
            BumpType::Patch => Version {
                major: self.major,
                minor: self.minor,
                patch: self.patch + 1,
            },
        }
    }

    pub fn to_tag(&self) -> String {
        format!("v{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl From<SemVersion> for Version {
    fn from(v: SemVersion) -> Self {
        Version {
            major: v.major as u32,
            minor: v.minor as u32,
            patch: v.patch as u32,
        }
    }
}

impl From<&Version> for SemVersion {
    fn from(v: &Version) -> Self {
        SemVersion::new(v.major as u64, v.minor as u64, v.patch as u64)
    }
}
