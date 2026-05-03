//! Version bumping utilities

use super::EditorError;
use super::Result;
use std::fmt;

/// Version bump type
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum BumpType {
    Major,
    Minor,
    Patch,
    PreRelease(String),
    Build(String),
}

/// Semantic version representation
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    /// Parse version from string like "1.2.3"
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

    /// Parse version from tag like "v1.2.3"
    #[allow(dead_code)]
    pub fn from_tag(tag: &str) -> Option<Self> {
        let tag = tag.trim();
        let version_str = tag.strip_prefix('v').unwrap_or(tag);
        Self::parse(version_str).ok()
    }

    /// Bump version according to bump type
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
            BumpType::PreRelease(_) | BumpType::Build(_) => self.clone(),
        }
    }

    /// Convert version to tag string like "v1.2.3"
    #[allow(dead_code)]
    pub fn to_tag(&self) -> String {
        format!("v{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Apply version bump to a version string
#[allow(dead_code)]
pub fn apply_bump(version: &str, bump_type: &BumpType) -> Result<String> {
    let v = Version::parse(version)?;

    match bump_type {
        BumpType::PreRelease(label) => {
            Ok(format!("{}.{}.{}-{}", v.major, v.minor, v.patch, label))
        }
        BumpType::Build(label) => Ok(format!("{}.{}.{}+{}", v.major, v.minor, v.patch, label)),
        _ => Ok(v.bump(bump_type).to_string()),
    }
}

/// Version editing configuration
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct EditorConfig {
    pub dry_run: bool,
    pub skip_push: bool,
    pub force: bool,
    pub message: Option<String>,
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

        let v = Version::from_tag("  v3.4.5  ").unwrap();
        assert_eq!(v.major, 3);
        assert_eq!(v.minor, 4);
        assert_eq!(v.patch, 5);

        assert!(Version::from_tag("invalid").is_none());
        assert!(Version::from_tag("v1.2").is_none());
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

        let bumped = v.bump(&BumpType::PreRelease("beta".to_string()));
        assert_eq!(bumped.major, 1);
        assert_eq!(bumped.minor, 2);
        assert_eq!(bumped.patch, 3);

        let bumped = v.bump(&BumpType::Build("123".to_string()));
        assert_eq!(bumped.major, 1);
        assert_eq!(bumped.minor, 2);
        assert_eq!(bumped.patch, 3);
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

    #[test]
    fn test_apply_bump() {
        assert_eq!(apply_bump("1.2.3", &BumpType::Major).unwrap(), "2.0.0");
        assert_eq!(apply_bump("1.2.3", &BumpType::Minor).unwrap(), "1.3.0");
        assert_eq!(apply_bump("1.2.3", &BumpType::Patch).unwrap(), "1.2.4");
        assert_eq!(
            apply_bump("1.2.3", &BumpType::PreRelease("beta".to_string())).unwrap(),
            "1.2.3-beta"
        );
        assert_eq!(
            apply_bump("1.2.3", &BumpType::Build("20240101".to_string())).unwrap(),
            "1.2.3+20240101"
        );

        assert!(apply_bump("invalid", &BumpType::Patch).is_err());
        assert!(apply_bump("1.2", &BumpType::Patch).is_err());
    }

    #[test]
    fn test_editor_config_default() {
        let config = EditorConfig::default();
        assert!(!config.dry_run);
        assert!(!config.skip_push);
        assert!(!config.force);
        assert!(config.message.is_none());
    }
}
