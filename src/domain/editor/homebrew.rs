use super::{
    EditorError, FileEditor, Result, VersionLocation, VersionPosition, preserve_line_endings,
};
use std::path::Path;

pub struct HomebrewFormulaEditor;

impl HomebrewFormulaEditor {
    fn find_version_position(content: &str) -> Option<VersionPosition> {
        let version_pattern = regex::Regex::new(r#"version\s+"([^"]+)""#).ok()?;

        if let Some(m) = version_pattern.find(content) {
            let start = m.start();
            let end = m.end();
            let line = content[..start].chars().filter(|&c| c == '\n').count() + 1;
            return Some(VersionPosition { start, end, line });
        }

        None
    }
}

impl FileEditor for HomebrewFormulaEditor {
    fn name(&self) -> &'static str {
        "homebrew"
    }

    fn file_patterns(&self) -> &[&str] {
        &["{parent}.rb"]
    }

    fn matches_file(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e == "rb")
            .unwrap_or(false)
    }

    fn parse(&self, content: &str) -> Result<VersionLocation> {
        let project_version = Self::find_version_position(content);

        if project_version.is_none() {
            return Err(EditorError::VersionNotFound(
                "Homebrew formula does not have version field".to_string(),
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
        if let Some(ref pos) = location.project_version {
            let mut result = String::new();
            result.push_str(&content[..pos.start]);
            result.push_str(&format!("version \"{}\"", new_version));
            result.push_str(&content[pos.end..]);
            Ok(preserve_line_endings(content, result))
        } else {
            Err(EditorError::VersionNotFound(
                "Homebrew formula does not have version field".to_string(),
            ))
        }
    }

    fn validate(&self, original: &str, edited: &str) -> Result<()> {
        // For Ruby files, just ensure we didn't corrupt the syntax
        if edited.contains("version \"\"") {
            return Err(EditorError::FormatPreservationError(
                "Homebrew formula version field is empty".to_string(),
            ));
        }

        Ok(())
    }
}
