use super::{
    EditorError, FileEditor, Result, VersionLocation, VersionPosition, replace_at_position,
};
use std::path::Path;

pub struct VersionTextEditor;

impl VersionTextEditor {
    fn find_version_position(content: &str) -> Option<VersionPosition> {
        let version_pattern = regex::Regex::new(r#"\d+\.\d+\.\d+"#).ok()?;

        if let Some(m) = version_pattern.find(content) {
            let start = m.start();
            let end = m.end();
            return Some(VersionPosition { start, end });
        }

        None
    }
}

impl FileEditor for VersionTextEditor {
    fn file_patterns(&self) -> &[&str] {
        &["version.txt", "VERSION", "VERSION.txt"]
    }

    fn matches_file(&self, path: &Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == "version.txt" || n == "VERSION" || n == "VERSION.txt")
            .unwrap_or(false)
    }

    fn parse(&self, content: &str) -> Result<VersionLocation> {
        let project_version = Self::find_version_position(content);

        if project_version.is_none() {
            return Err(EditorError::VersionNotFound(
                "No version found in text file".to_string(),
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
                "No version position found".to_string(),
            ))
        }
    }

    fn validate(&self, original: &str, edited: &str) -> Result<()> {
        if original.is_empty() && edited.is_empty() {
            return Ok(());
        }

        if edited.len().abs_diff(original.len()) > original.len() / 2 {
            return Err(EditorError::FormatPreservationError(
                "Text file changed too much".to_string(),
            ));
        }

        Ok(())
    }
}
