use super::{EditorError, FileEditor, Result, VersionPosition, find_version_value_in_quotes};

pub struct PackageJsonEditor;

impl PackageJsonEditor {
    fn find_version_position(content: &str) -> Option<VersionPosition> {
        let pattern = regex::Regex::new(r#""version"\s*:\s*"([^"]*)""#).ok()?;
        find_version_value_in_quotes(content, &pattern)
    }
}

impl FileEditor for PackageJsonEditor {
    fn name(&self) -> &str {
        "package.json"
    }

    fn file_patterns(&self) -> &[&str] {
        &["package.json"]
    }

    fn find_version(&self, content: &str) -> Option<VersionPosition> {
        Self::find_version_position(content)
    }

    fn parse(&self, content: &str) -> Result<super::VersionLocation> {
        let _: serde_json::Value = serde_json::from_str(content).map_err(|e| {
            EditorError::ParseError(format!("Failed to parse package.json: {}", e))
        })?;

        let project_version = self.find_version(content);

        if project_version.is_none() {
            return Err(EditorError::VersionNotFound(
                "package.json does not have version field".to_string(),
            ));
        }

        Ok(super::VersionLocation {
            project_version,
            is_workspace_root: false,
        })
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
