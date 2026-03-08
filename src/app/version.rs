#[derive(Debug, Clone, Default)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub fn from_tag(tag: &str) -> Option<Self> {
        let tag = tag.trim();
        let tag = tag.strip_prefix('v').unwrap_or(tag);
        let parts: Vec<&str> = tag.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        Some(Version {
            major: parts[0].parse().ok()?,
            minor: parts[1].parse().ok()?,
            patch: parts[2].parse().ok()?,
        })
    }

    pub fn bump(&self, bump_type: &str) -> Self {
        match bump_type {
            "major" => Version {
                major: self.major + 1,
                minor: 0,
                patch: 0,
            },
            "minor" => Version {
                major: self.major,
                minor: self.minor + 1,
                patch: 0,
            },
            _ => Version {
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

pub fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let va = Version::from_tag(a);
    let vb = Version::from_tag(b);
    match (va, vb) {
        (Some(va), Some(vb)) => {
            if va.major != vb.major {
                vb.major.cmp(&va.major)
            } else if va.minor != vb.minor {
                vb.minor.cmp(&va.minor)
            } else {
                vb.patch.cmp(&va.patch)
            }
        }
        _ => b.cmp(a),
    }
}
