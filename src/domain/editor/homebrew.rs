use super::{EditorError, FileEditor, Result, VersionPosition, extract_version_position};
use std::path::Path;

pub struct HomebrewFormulaEditor;

impl HomebrewFormulaEditor {
    fn find_version_position(content: &str) -> Option<VersionPosition> {
        let pattern = regex::Regex::new(r#"version\s+"([^"]+)""#).ok()?;
        extract_version_position(content, &pattern)
    }
}

impl FileEditor for HomebrewFormulaEditor {
    fn name(&self) -> &str {
        "Homebrew formula"
    }

    fn file_patterns(&self) -> &[&str] {
        &["{parent}.rb"]
    }

    fn find_version(&self, content: &str) -> Option<VersionPosition> {
        Self::find_version_position(content)
    }

    fn matches_file(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e == "rb")
            .unwrap_or(false)
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
