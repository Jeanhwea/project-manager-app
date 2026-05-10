use super::{
    EditorError, FileEditor, Result, VersionLocation, VersionPosition,
    find_version_value_in_quotes, replace_at_position,
};
use std::path::Path;

pub struct PackageJsonEditor;

impl PackageJsonEditor {
    fn find_version_position(content: &str) -> Option<VersionPosition> {
        let pattern = regex::Regex::new(r#""version"\s*:\s*"([^"]*)""#).ok()?;
        find_version_value_in_quotes(content, &pattern)
    }
}

impl FileEditor for PackageJsonEditor {
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
        let _: serde_json::Value = serde_json::from_str(content).map_err(|e| {
            EditorError::ParseError(format!("Failed to parse package.json: {}", e))
        })?;

        let project_version = Self::find_version_position(content);

        if project_version.is_none() {
            return Err(EditorError::VersionNotFound(
                "package.json does not have version field".to_string(),
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
        if let Some(ref pos) = location.project_version {
            Ok(replace_at_position(content, pos, new_version))
        } else {
            Err(EditorError::VersionNotFound(
                "package.json does not have version field".to_string(),
            ))
        }
    }

    fn validate(&self, original: &str, edited: &str) -> Result<()> {
        if serde_json::from_str::<serde_json::Value>(edited).is_err() {
            return Err(EditorError::FormatPreservationError(
                "package.json format validation failed".to_string(),
            ));
        }

        let original_len = original.len();
        let edited_len = edited.len();
        if edited_len.abs_diff(original_len) > original_len / 2 {
            return Err(EditorError::FormatPreservationError(
                "package.json changed too much".to_string(),
            ));
        }

        Ok(())
    }
}
