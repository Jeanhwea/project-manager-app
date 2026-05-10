mod cargo_toml;
mod cmake;
mod homebrew;
mod package_json;
mod pom_xml;
mod project_py;
mod pyproject;
mod version_bump;
mod version_text;

use cargo_toml::CargoTomlEditor;
use cmake::CMakeListsEditor;
use homebrew::HomebrewFormulaEditor;
use package_json::PackageJsonEditor;
use pom_xml::PomXmlEditor;
use project_py::PythonVersionEditor;
use pyproject::PyprojectEditor;
pub use version_bump::{BumpType, Version};
use version_text::VersionTextEditor;

use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum EditorError {
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Write error: {0}")]
    WriteError(#[from] std::io::Error),

    #[error("Version format error: {0}")]
    VersionFormatError(String),

    #[error("Version not found: {0}")]
    VersionNotFound(String),

    #[error("Format preservation error: {0}")]
    FormatPreservationError(String),
}

pub type Result<T> = std::result::Result<T, EditorError>;

pub trait FileEditor: Send + Sync {
    fn file_patterns(&self) -> &[&str];
    fn matches_file(&self, path: &Path) -> bool;
    fn parse(&self, content: &str) -> Result<VersionLocation>;
    fn edit(
        &self,
        content: &str,
        location: &VersionLocation,
        new_version: &str,
    ) -> Result<String>;
    fn validate(&self, original: &str, edited: &str) -> Result<()>;
}

#[derive(Debug, Clone, Default)]
pub struct VersionLocation {
    pub project_version: Option<VersionPosition>,
    pub is_workspace_root: bool,
}

#[derive(Debug, Clone)]
pub struct VersionPosition {
    pub start: usize,
    pub end: usize,
}

pub struct EditorRegistry {
    editors: Vec<Box<dyn FileEditor>>,
}

impl EditorRegistry {
    pub fn new() -> Self {
        Self {
            editors: Vec::new(),
        }
    }

    pub fn default_with_editors() -> Self {
        Self::new()
            .register(CargoTomlEditor)
            .register(PackageJsonEditor)
            .register(VersionTextEditor)
            .register(CMakeListsEditor)
            .register(HomebrewFormulaEditor)
            .register(PomXmlEditor)
            .register(PythonVersionEditor)
            .register(PyprojectEditor)
    }

    pub fn register(mut self, editor: impl FileEditor + 'static) -> Self {
        self.editors.push(Box::new(editor));
        self
    }

    pub fn detect_editor(&self, path: &Path) -> Option<&dyn FileEditor> {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        for editor in &self.editors {
            for pattern in editor.file_patterns() {
                if pattern.contains("{parent}") {
                    let parent_dir = path
                        .parent()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    let replaced = pattern.replace("{parent}", parent_dir);
                    if file_name == replaced || path.ends_with(&replaced) {
                        return Some(editor.as_ref());
                    }
                } else if file_name == *pattern || path.ends_with(pattern) {
                    return Some(editor.as_ref());
                }
            }
        }

        for editor in &self.editors {
            if editor.matches_file(path) {
                return Some(editor.as_ref());
            }
        }

        None
    }
}

impl Default for EditorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn write_with_backup(path: &str, content: &str) -> Result<()> {
    let backup_path = format!("{}.bak", path);

    std::fs::copy(path, &backup_path).map_err(EditorError::WriteError)?;

    match std::fs::write(path, content) {
        Ok(_) => {
            let _ = std::fs::remove_file(&backup_path);
            Ok(())
        }
        Err(e) => {
            let _ = std::fs::rename(&backup_path, path);
            Err(EditorError::WriteError(e))
        }
    }
}

pub fn preserve_line_endings(original: &str, edited: String) -> String {
    let original_has_crlf = original.contains("\r\n");
    let edited_has_crlf = edited.contains("\r\n");

    if original_has_crlf && !edited_has_crlf {
        edited.replace("\n", "\r\n")
    } else if !original_has_crlf && edited_has_crlf {
        edited.replace("\r\n", "\n")
    } else {
        edited
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_registry_detects_cargo_toml() {
        let registry = EditorRegistry::default_with_editors();
        assert!(registry.detect_editor(Path::new("Cargo.toml")).is_some());
        assert!(registry.detect_editor(Path::new("package.json")).is_some());
        assert!(registry.detect_editor(Path::new("unknown.xyz")).is_none());
    }

    #[test]
    fn test_preserve_line_endings() {
        let original_crlf = "line1\r\nline2\r\n";
        let original_lf = "line1\nline2\n";

        let edited = "line1\nline2\n";

        assert_eq!(
            preserve_line_endings(original_crlf, edited.to_string()),
            "line1\r\nline2\r\n"
        );
        assert_eq!(
            preserve_line_endings(original_lf, edited.to_string()),
            "line1\nline2\n"
        );
    }

    #[test]
    fn test_cargo_toml_editor() {
        let content = r#"[package]
name = "test"
version = "1.2.3"

[dependencies]
serde = "1.0""#;

        let editor = CargoTomlEditor;
        let location = editor.parse(content).unwrap();

        assert!(location.project_version.is_some());
        assert!(!location.is_workspace_root);

        let edited = editor.edit(content, &location, "2.0.0").unwrap();
        assert!(edited.contains("version = \"2.0.0\""));
        assert!(!edited.contains("version = \"1.2.3\""));
    }

    #[test]
    fn test_package_json_editor() {
        let content = r#"{
  "name": "test",
  "version": "1.2.3",
  "dependencies": {
    "lodash": "^4.17.0"
  }
}"#;

        let editor = PackageJsonEditor;
        let location = editor.parse(content).unwrap();

        assert!(location.project_version.is_some());

        let edited = editor.edit(content, &location, "2.0.0").unwrap();
        assert!(edited.contains("\"version\": \"2.0.0\""));
        assert!(!edited.contains("\"version\": \"1.2.3\""));
    }
}
