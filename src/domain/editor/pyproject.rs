use super::{
    EditorError, FileEditor, Result, VersionLocation, VersionPosition, replace_at_position,
};
use std::path::Path;

pub struct PyprojectEditor;

impl PyprojectEditor {
    fn find_version_in_section(content: &str, section_header: &str) -> Option<VersionPosition> {
        let section_start = content.find(section_header)?;
        let section_end = content[section_start..]
            .find("\n[")
            .map(|p| section_start + p)
            .unwrap_or(content.len());

        let section_content = &content[section_start..section_end];

        let pattern = regex::Regex::new(r#"version\s*=\s*"([^"]*)""#).ok()?;
        let caps = pattern.captures(section_content)?;
        let version_match = caps.get(1)?;

        Some(VersionPosition {
            start: section_start + version_match.start(),
            end: section_start + version_match.end(),
        })
    }
}

impl FileEditor for PyprojectEditor {
    fn file_patterns(&self) -> &[&str] {
        &["pyproject.toml"]
    }

    fn matches_file(&self, path: &Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == "pyproject.toml")
            .unwrap_or(false)
    }

    fn parse(&self, content: &str) -> Result<VersionLocation> {
        let doc = content.parse::<toml_edit::DocumentMut>().map_err(|e| {
            EditorError::ParseError(format!("Failed to parse pyproject.toml: {}", e))
        })?;

        if doc.contains_key("project") {
            let project_version = Self::find_version_in_section(content, "[project]");
            if project_version.is_some() {
                return Ok(VersionLocation {
                    project_version,
                    is_workspace_root: false,
                });
            }
        }

        if doc.contains_key("tool")
            && let Some(tool) = doc.get("tool")
            && let Some(tool_table) = tool.as_table_like()
            && tool_table.contains_key("poetry")
        {
            let project_version = Self::find_version_in_section(content, "[tool.poetry]");
            if project_version.is_some() {
                return Ok(VersionLocation {
                    project_version,
                    is_workspace_root: false,
                });
            }
        }

        Err(EditorError::VersionNotFound(
            "pyproject.toml does not have version field in [project] or [tool.poetry] section"
                .to_string(),
        ))
    }

    fn edit(
        &self,
        content: &str,
        location: &VersionLocation,
        new_version: &str,
    ) -> Result<String> {
        if let Some(ref pos) = location.project_version {
            Ok(replace_at_position(content, pos, new_version))
        } else {
            Err(EditorError::VersionNotFound(
                "pyproject.toml does not have version field".to_string(),
            ))
        }
    }

    fn validate(&self, _original: &str, edited: &str) -> Result<()> {
        if edited.parse::<toml_edit::DocumentMut>().is_err() {
            return Err(EditorError::FormatPreservationError(
                "pyproject.toml format validation failed".to_string(),
            ));
        }
        Ok(())
    }
}
