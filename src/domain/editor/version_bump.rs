//! Version bumping utilities for the editor module

use super::EditorError;
use super::Result;

/// Version bump type
#[derive(Debug, Clone, PartialEq)]
pub enum BumpType {
    Major,
    Minor,
    Patch,
    PreRelease(String),
    Build(String),
}

/// Apply version bump to a version string
pub fn apply_bump(version: &str, bump_type: &BumpType) -> Result<String> {
    let mut parts = version.split('.');
    let major = parts.next().and_then(|s| s.parse::<u32>().ok());
    let minor = parts.next().and_then(|s| s.parse::<u32>().ok());
    let patch = parts.next().and_then(|s| s.parse::<u32>().ok());

    match (major, minor, patch) {
        (Some(major), Some(minor), Some(patch)) => {
            let (new_major, new_minor, new_patch) = match bump_type {
                BumpType::Major => (major + 1, 0, 0),
                BumpType::Minor => (major, minor + 1, 0),
                BumpType::Patch => (major, minor, patch + 1),
                BumpType::PreRelease(label) => {
                    return Ok(format!("{}.{}.{}-{}", major, minor, patch, label));
                }
                BumpType::Build(label) => {
                    return Ok(format!("{}.{}.{}+{}", major, minor, patch, label));
                }
            };
            Ok(format!("{}.{}.{}", new_major, new_minor, new_patch))
        }
        _ => Err(EditorError::VersionFormatError(format!(
            "Invalid version format: {}",
            version
        ))),
    }
}

/// Version editing configuration
#[derive(Debug, Clone)]
pub struct EditorConfig {
    pub dry_run: bool,
    pub skip_push: bool,
    pub force: bool,
    pub message: Option<String>,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            dry_run: false,
            skip_push: false,
            force: false,
            message: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        // Test invalid versions
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
