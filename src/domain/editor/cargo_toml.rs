use super::{
    EditorError, FileEditor, Result, VersionLocation, VersionPosition, preserve_line_endings,
};
use std::path::Path;

pub struct CargoTomlEditor;

impl CargoTomlEditor {
    fn find_version_position(
        content: &str,
        doc: &toml_edit::DocumentMut,
    ) -> Option<VersionPosition> {
        let package = doc.get("package")?.as_table_like()?;

        if !package.contains_key("version") {
            return None;
        }

        let version_pattern = regex::Regex::new(r#"version\s*=\s*"[^"]*""#).ok()?;

        let package_start = content.find("[package]")?;
        let package_end = content[package_start..]
            .find("\n[")
            .map(|p| package_start + p)
            .unwrap_or(content.len());

        let package_section = &content[package_start..package_end];

        if let Some(m) = version_pattern.find(package_section) {
            let start = package_start + m.start();
            let end = package_start + m.end();
            let line = content[..start].chars().filter(|&c| c == '\n').count() + 1;
            return Some(VersionPosition { start, end, line });
        }

        None
    }
}

impl FileEditor for CargoTomlEditor {
    fn name(&self) -> &'static str {
        "cargo_toml"
    }

    fn file_patterns(&self) -> &[&str] {
        &["Cargo.toml"]
    }

    fn matches_file(&self, path: &Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == "Cargo.toml")
            .unwrap_or(false)
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
                parent_version: None,
                is_workspace_root: true,
                dependency_refs: Vec::new(),
            });
        }

        if !has_package {
            return Err(EditorError::VersionNotFound(
                "Cargo.toml does not have [package] section".to_string(),
            ));
        }

        let project_version = Self::find_version_position(content, &doc);

        if project_version.is_none() {
            return Err(EditorError::VersionNotFound(
                "Cargo.toml [package] section does not have version field".to_string(),
            ));
        }

        Ok(VersionLocation {
            project_version,
            parent_version: None,
            is_workspace_root: false,
            dependency_refs: Vec::new(),
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

        let mut doc = content
            .parse::<toml_edit::DocumentMut>()
            .map_err(|e| EditorError::ParseError(format!("Failed to parse Cargo.toml: {}", e)))?;

        if let Some(package) = doc.get_mut("package")
            && let Some(table) = package.as_table_like_mut()
        {
            table.insert("version", toml_edit::value(new_version));
        }

        let edited = doc.to_string();
        Ok(preserve_line_endings(content, edited))
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
