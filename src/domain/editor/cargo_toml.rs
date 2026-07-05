use super::{
    EditorError, FileEditor, Result, VersionLocation, VersionPosition, replace_at_position,
};

pub struct CargoTomlEditor;

impl CargoTomlEditor {
    fn find_version_position(content: &str) -> Option<VersionPosition> {
        find_version_in_section(content, "[package]")
            .or_else(|| find_version_in_section(content, "[workspace.package]"))
    }
}

fn find_version_in_section(content: &str, section_header: &str) -> Option<VersionPosition> {
    let section_start = content.find(section_header)?;
    let section_end = content[section_start + section_header.len()..]
        .find("\n[")
        .map(|p| section_start + section_header.len() + p)
        .unwrap_or(content.len());

    let section = &content[section_start..section_end];

    let pattern = regex::Regex::new(r#"version\s*=\s*"([^"]*)""#).ok()?;
    let caps = pattern.captures(section)?;
    let version_match = caps.get(1)?;

    Some(VersionPosition {
        start: section_start + version_match.start(),
        end: section_start + version_match.end(),
    })
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

        let workspace_has_package_version = has_workspace
            && doc
                .get("workspace")
                .and_then(|w| w.as_table())
                .and_then(|t| t.get("package"))
                .and_then(|p| p.as_table())
                .map(|pkg| pkg.contains_key("version"))
                .unwrap_or(false);

        if !has_package && has_workspace {
            if workspace_has_package_version {
                let project_version = self.find_version(content);
                if project_version.is_none() {
                    return Err(EditorError::VersionNotFound(
                        "Cargo.toml [workspace.package] section does not have version field"
                            .to_string(),
                    ));
                }
                return Ok(VersionLocation {
                    project_version,
                    is_workspace_root: false,
                });
            }
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
