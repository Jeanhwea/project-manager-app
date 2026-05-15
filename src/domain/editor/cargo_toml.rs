use super::{
    EditorError, FileEditor, Result, VersionLocation, VersionPosition, replace_at_position,
};

pub struct CargoTomlEditor;

impl CargoTomlEditor {
    fn find_version_position(content: &str) -> Option<VersionPosition> {
        let package_start = content.find("[package]")?;
        let package_end = content[package_start..]
            .find("\n[")
            .map(|p| package_start + p)
            .unwrap_or(content.len());

        let package_section = &content[package_start..package_end];

        let pattern = regex::Regex::new(r#"version\s*=\s*"([^"]*)""#).ok()?;
        let caps = pattern.captures(package_section)?;
        let version_match = caps.get(1)?;

        Some(VersionPosition {
            start: package_start + version_match.start(),
            end: package_start + version_match.end(),
        })
    }
}

impl FileEditor for CargoTomlEditor {
    fn name(&self) -> &str {
        "Cargo.toml"
    }

    fn file_patterns(&self) -> &[&str] {
        &["Cargo.toml"]
    }

    fn candidate_files(&self) -> Vec<&str> {
        vec!["Cargo.toml", "src-tauri/Cargo.toml"]
    }

    fn find_version(&self, content: &str) -> Option<VersionPosition> {
        Self::find_version_position(content)
    }

    fn parse(&self, content: &str) -> Result<VersionLocation> {
        let doc = content
            .parse::<toml_edit::DocumentMut>()
            .map_err(|e| EditorError::ParseError(format!("Failed to parse Cargo.toml: {}", e)))?;

        let has_package = doc.contains_key("package");
        let has_workspace = doc.contains_key("workspace");

        if !has_package && has_workspace {
            return Ok(VersionLocation {
                project_version: None,
                is_workspace_root: true,
            });
        }

        if !has_package {
            return Err(EditorError::VersionNotFound(
                "Cargo.toml does not have [package] section".to_string(),
            ));
        }

        let project_version = self.find_version(content);

        if project_version.is_none() {
            return Err(EditorError::VersionNotFound(
                "Cargo.toml [package] section does not have version field".to_string(),
            ));
        }

        Ok(VersionLocation {
            project_version,
            is_workspace_root: false,
        })
    }

    fn edit(
        &self,
        content: &str,
        location: &VersionLocation,
        new_version: &str,
    ) -> Result<String> {
        if location.is_workspace_root {
            return Err(EditorError::VersionNotFound(
                "Cargo.toml is a workspace root file, no project version".to_string(),
            ));
        }

        if let Some(ref pos) = location.project_version {
            Ok(replace_at_position(content, pos, new_version))
        } else {
            Err(EditorError::VersionNotFound(
                "Cargo.toml does not have version field".to_string(),
            ))
        }
    }

    fn validate(&self, _original: &str, edited: &str) -> Result<()> {
        if edited.parse::<toml_edit::DocumentMut>().is_err() {
            return Err(EditorError::FormatPreservationError(
                "Cargo.toml format validation failed".to_string(),
            ));
        }
        Ok(())
    }
}
