use super::{EditorError, FileEditor, Result, VersionPosition, extract_version_position};
use std::path::Path;

pub struct PythonVersionEditor;

impl PythonVersionEditor {
    fn find_version_position(content: &str) -> Option<VersionPosition> {
        let pattern = regex::Regex::new(r#"__version__\s*=\s*["']([^"']+)["']"#).ok()?;
        extract_version_position(content, &pattern)
    }
}

impl FileEditor for PythonVersionEditor {
    fn name(&self) -> &str {
        "Python version file"
    }

    fn file_patterns(&self) -> &[&str] {
        &["__init__.py", "version.py", "__version__.py"]
    }

    fn candidate_files(&self) -> Vec<&str> {
        vec![
            "__init__.py",
            "version.py",
            "__version__.py",
            "{}/__version__.py",
        ]
    }

    fn matches_file(&self, path: &Path) -> bool {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        self.file_patterns().iter().any(|pattern| {
            if pattern.contains("{parent}") {
                let parent_dir = path
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                let replaced = pattern.replace("{parent}", parent_dir);
                file_name == replaced || path.ends_with(&replaced)
            } else if pattern.contains("{}") {
                // Handle the {} pattern for directory matching
                if let Some(parent) = path.parent() {
                    let parent_name = parent.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    let replaced = pattern.replace("{}", parent_name);
                    file_name == replaced || path.ends_with(&replaced)
                } else {
                    false
                }
            } else {
                file_name == *pattern || path.ends_with(pattern)
            }
        })
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
