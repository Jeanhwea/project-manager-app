mod cargo_toml;
mod cmake;
mod detect;
mod homebrew;
mod package_json;
mod pom_xml;
mod project_py;
mod pyproject;
mod version_bump;
mod version_text;

pub use detect::{add_lockfile_operations, compute_edited_content, resolve_config_files};

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
    fn name(&self) -> &str;
    fn file_patterns(&self) -> &[&str];
    fn find_version(&self, content: &str) -> Option<VersionPosition>;

    fn candidate_files(&self) -> Vec<&str> {
        self.file_patterns().to_vec()
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
                // Handle {} pattern for directory matching
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

    fn parse(&self, content: &str) -> Result<VersionLocation> {
        let project_version = self.find_version(content);
        if project_version.is_none() {
            return Err(EditorError::VersionNotFound(format!(
                "{} does not have version field",
                self.name()
            )));
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
            Err(EditorError::VersionNotFound(format!(
                "{} does not have version field",
                self.name()
            )))
        }
    }

    fn validate(&self, _original: &str, _edited: &str) -> Result<()> {
        Ok(())
    }
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

impl std::fmt::Debug for EditorRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EditorRegistry")
            .field("editors", &self.editors.len())
            .finish()
    }
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
        self.editors
            .iter()
            .find(|editor| editor.matches_file(path))
            .map(|e| e.as_ref())
    }

    pub fn candidate_files(&self) -> Vec<&str> {
        self.editors
            .iter()
            .flat_map(|editor| editor.candidate_files())
            .collect()
    }
}

impl Default for EditorRegistry {
    fn default() -> Self {
        Self::default_with_editors()
    }
}

pub fn write_with_backup(path: &str, content: &str) -> Result<()> {
    let tmp_path = format!("{}.tmp", path);
    std::fs::write(&tmp_path, content).map_err(EditorError::WriteError)?;
    std::fs::rename(&tmp_path, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp_path);
        EditorError::WriteError(e)
    })
}

pub fn replace_at_position(content: &str, pos: &VersionPosition, new_value: &str) -> String {
    let mut result = String::with_capacity(content.len() + new_value.len());
    result.push_str(&content[..pos.start]);
    result.push_str(new_value);
    result.push_str(&content[pos.end..]);
    result
}

pub fn extract_version_position(
    content: &str,
    pattern: &regex::Regex,
) -> Option<VersionPosition> {
    let caps = pattern.captures(content)?;
    let version_match = caps.get(1)?;
    Some(VersionPosition {
        start: version_match.start(),
        end: version_match.end(),
    })
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
        assert!(edited.contains("name = \"test\""));
        assert!(edited.contains("serde = \"1.0\""));
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
        assert!(edited.contains("\"name\": \"test\""));
        assert!(edited.contains("\"lodash\": \"^4.17.0\""));
        assert!(edited.contains("\"dependencies\""));
    }

    #[test]
    fn test_package_json_preserves_key_order() {
        let content = r#"{
  "name": "test",
  "private": true,
  "version": "1.2.3",
  "scripts": {
    "dev": "vite"
  }
}"#;

        let editor = PackageJsonEditor;
        let location = editor.parse(content).unwrap();
        let edited = editor.edit(content, &location, "2.0.0").unwrap();

        let name_pos = edited.find("\"name\"").unwrap();
        let private_pos = edited.find("\"private\"").unwrap();
        let version_pos = edited.find("\"version\"").unwrap();
        let scripts_pos = edited.find("\"scripts\"").unwrap();

        assert!(name_pos < private_pos, "key order should be preserved");
        assert!(private_pos < version_pos, "key order should be preserved");
        assert!(version_pos < scripts_pos, "key order should be preserved");
    }

    #[test]
    fn test_replace_at_position() {
        let content = "version = \"1.2.3\"";
        let pos = VersionPosition { start: 11, end: 16 };
        let result = replace_at_position(content, &pos, "2.0.0");
        assert_eq!(result, "version = \"2.0.0\"");
    }
}
