use super::EditorError;
use super::Result;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum BumpType {
    Major,
    Minor,
    Patch,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub fn parse(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.trim().split('.').collect();
        if parts.len() != 3 {
            return Err(EditorError::VersionFormatError(format!(
                "Invalid version format: {}",
                s
            )));
        }

        let major = parts[0].parse::<u32>().map_err(|_| {
            EditorError::VersionFormatError(format!("Invalid major version: {}", parts[0]))
        })?;
        let minor = parts[1].parse::<u32>().map_err(|_| {
            EditorError::VersionFormatError(format!("Invalid minor version: {}", parts[1]))
        })?;
        let patch = parts[2].parse::<u32>().map_err(|_| {
            EditorError::VersionFormatError(format!("Invalid patch version: {}", parts[2]))
        })?;

        Ok(Version {
            major,
            minor,
            patch,
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

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parse() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);

        let v = Version::parse("  2.3.4  ").unwrap();
        assert_eq!(v.major, 2);
        assert_eq!(v.minor, 3);
        assert_eq!(v.patch, 4);

        assert!(Version::parse("invalid").is_err());
        assert!(Version::parse("1.2").is_err());
        assert!(Version::parse("1.2.3.4").is_err());
    }

    #[test]
    fn test_version_from_tag() {
        let v = Version::from_tag("v1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);

        let v = Version::from_tag("2.3.4").unwrap();
        assert_eq!(v.major, 2);
        assert_eq!(v.minor, 3);
        assert_eq!(v.patch, 4);
    }

    #[test]
    fn test_version_bump() {
        let v = Version {
            major: 1,
            minor: 2,
            patch: 3,
        };

        let bumped = v.bump(&BumpType::Major);
        assert_eq!(bumped.major, 2);
        assert_eq!(bumped.minor, 0);
        assert_eq!(bumped.patch, 0);

        let bumped = v.bump(&BumpType::Minor);
        assert_eq!(bumped.major, 1);
        assert_eq!(bumped.minor, 3);
        assert_eq!(bumped.patch, 0);

        let bumped = v.bump(&BumpType::Patch);
        assert_eq!(bumped.major, 1);
        assert_eq!(bumped.minor, 2);
        assert_eq!(bumped.patch, 4);
    }

    #[test]
    fn test_version_to_tag() {
        let v = Version {
            major: 1,
            minor: 2,
            patch: 3,
        };
        assert_eq!(v.to_tag(), "v1.2.3");
    }

    #[test]
    fn test_version_display() {
        let v = Version {
            major: 1,
            minor: 2,
            patch: 3,
        };
        assert_eq!(format!("{}", v), "1.2.3");
    }
}
