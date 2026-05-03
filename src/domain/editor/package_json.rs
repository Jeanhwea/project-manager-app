use super::{
    EditorError, FileEditor, Result, VersionLocation, VersionPosition, preserve_line_endings,
};
use std::path::Path;

pub struct PackageJsonEditor;

impl PackageJsonEditor {
    fn find_version_position(content: &str) -> Option<VersionPosition> {
        let version_pattern = regex::Regex::new(r#""version"\s*:\s*"[^"]*""#).ok()?;

        if let Some(m) = version_pattern.find(content) {
            let start = m.start();
            let end = m.end();
            let line = content[..start].chars().filter(|&c| c == '\n').count() + 1;
            return Some(VersionPosition { start, end, line });
        }

        None
    }
}

impl FileEditor for PackageJsonEditor {
    fn name(&self) -> &'static str {
        "package_json"
    }

    fn file_patterns(&self) -> &[&str] {
        &["package.json"]
    }

    fn matches_file(&self, path: &Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == "package.json")
            .unwrap_or(false)
    }

    fn parse(&self, content: &str) -> Result<VersionLocation> {
        let json: serde_json::Value = serde_json::from_str(content).map_err(|e| {
            EditorError::ParseError(format!("Failed to parse package.json: {}", e))
        })?;

        if !json.is_object() {
            return Err(EditorError::ParseError(
                "package.json is not a JSON object".to_string(),
            ));
        }

        let project_version = Self::find_version_position(content);

        if project_version.is_none() {
            return Err(EditorError::VersionNotFound(
                "package.json does not have version field".to_string(),
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
        _location: &VersionLocation,
        new_version: &str,
    ) -> Result<String> {
        let mut json: serde_json::Value = serde_json::from_str(content).map_err(|e| {
            EditorError::ParseError(format!("Failed to parse package.json: {}", e))
        })?;

        if let Some(obj) = json.as_object_mut() {
            obj.insert(
                "version".to_string(),
                serde_json::Value::String(new_version.to_string()),
            );
        }

        let edited = serde_json::to_string_pretty(&json).map_err(|e| {
            EditorError::ParseError(format!("Failed to serialize package.json: {}", e))
        })?;

        Ok(preserve_line_endings(content, edited))
    }

    fn validate(&self, _original: &str, edited: &str) -> Result<()> {
        if serde_json::from_str::<serde_json::Value>(edited).is_err() {
            return Err(EditorError::FormatPreservationError(
                "package.json format validation failed".to_string(),
            ));
        }
        Ok(())
    }
}
