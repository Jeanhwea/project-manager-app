use super::{EditorError, FileEditor, Result, VersionPosition, find_version_value_in_quotes};

pub struct PythonVersionEditor;

impl PythonVersionEditor {
    fn find_version_position(content: &str) -> Option<VersionPosition> {
        let pattern = regex::Regex::new(r#"__version__\s*=\s*["']([^"']+)["']"#).ok()?;
        find_version_value_in_quotes(content, &pattern)
    }
}

impl FileEditor for PythonVersionEditor {
    fn name(&self) -> &str {
        "Python version file"
    }

    fn file_patterns(&self) -> &[&str] {
        &["__init__.py", "version.py", "__version__.py"]
    }

    fn find_version(&self, content: &str) -> Option<VersionPosition> {
        Self::find_version_position(content)
    }

    fn validate(&self, _original: &str, edited: &str) -> Result<()> {
        if edited.contains("__version__ = \"\"") {
            return Err(EditorError::FormatPreservationError(
                "Python version field is empty".to_string(),
            ));
        }

        Ok(())
    }
}
