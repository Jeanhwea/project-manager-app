use super::{EditorError, FileEditor, Result, VersionPosition};

pub struct VersionTextEditor;

impl FileEditor for VersionTextEditor {
    fn name(&self) -> &str {
        "version text file"
    }

    fn file_patterns(&self) -> &[&str] {
        &["version.txt", "VERSION", "VERSION.txt"]
    }

    fn find_version(&self, content: &str) -> Option<VersionPosition> {
        let version_pattern = regex::Regex::new(r#"\d+\.\d+\.\d+"#).ok()?;

        if let Some(m) = version_pattern.find(content) {
            return Some(VersionPosition {
                start: m.start(),
                end: m.end(),
            });
        }

        None
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
