use super::{
    EditorError, FileEditor, Result, VersionLocation, VersionPosition,
    find_version_value_in_quotes, replace_at_position,
};
use std::path::Path;

pub struct HomebrewFormulaEditor;

impl HomebrewFormulaEditor {
    fn find_version_position(content: &str) -> Option<VersionPosition> {
        let pattern = regex::Regex::new(r#"version\s+"([^"]+)""#).ok()?;
        find_version_value_in_quotes(content, &pattern)
    }
}

impl FileEditor for HomebrewFormulaEditor {
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
                "Homebrew formula does not have version field".to_string(),
            ))
        }
    }

    fn validate(&self, _original: &str, edited: &str) -> Result<()> {
        if edited.contains("version \"\"") {
            return Err(EditorError::FormatPreservationError(
                "Homebrew formula version field is empty".to_string(),
            ));
        }

        Ok(())
    }
}
