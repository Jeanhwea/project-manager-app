use super::{
    EditorError, FileEditor, Result, VersionLocation, VersionPosition, preserve_line_endings,
};
use std::path::Path;

pub struct VersionTextEditor;

impl VersionTextEditor {
    fn find_version_position(content: &str) -> Option<VersionPosition> {
        // Look for semantic version pattern
        let version_pattern = regex::Regex::new(r#"\d+\.\d+\.\d+"#).ok()?;

        if let Some(m) = version_pattern.find(content) {
            let start = m.start();
            let end = m.end();
            let line = content[..start].chars().filter(|&c| c == '\n').count() + 1;
            return Some(VersionPosition { start, end, line });
        }

        None
    }
}

impl FileEditor for VersionTextEditor {
    fn name(&self) -> &'static str {
        "version_text"
    }

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
        if let Some(version_pos) = &location.project_version {
            let mut edited = String::with_capacity(content.len());
            edited.push_str(&content[..version_pos.start]);
            edited.push_str(new_version);
            edited.push_str(&content[version_pos.end..]);
            Ok(preserve_line_endings(content, edited))
        } else {
            Err(EditorError::VersionNotFound(
                "No version position found".to_string(),
            ))
        }
    }

    fn validate(&self, original: &str, edited: &str) -> Result<()> {
        // For text files, just ensure we didn't corrupt the file
        if original.len() == 0 && edited.len() == 0 {
            return Ok(());
        }

        if edited.len() < original.len() / 2 || edited.len() > original.len() * 2 {
            return Err(EditorError::FormatPreservationError(
                "Text file changed too much".to_string(),
            ));
        }

        Ok(())
    }
}
