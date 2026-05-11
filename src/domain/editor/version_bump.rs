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
    pub fn parse(s: &str) -> super::Result<Self> {
        let ver = SemVersion::parse(s.trim()).map_err(|e| {
            super::EditorError::VersionFormatError(format!("Invalid version format: {}", e))
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

    #[test]
    fn test_version_from_semver() {
        let sem = SemVersion::parse("1.2.3").unwrap();
        let v: Version = sem.into();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_version_to_semver() {
        let v = Version {
            major: 1,
            minor: 2,
            patch: 3,
        };
        let sem: SemVersion = (&v).into();
        assert_eq!(sem, SemVersion::new(1, 2, 3));
    }
}
