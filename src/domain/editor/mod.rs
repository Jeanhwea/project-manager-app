mod cargo_toml;
mod cmake;
mod file_types;
mod homebrew;
mod package_json;
mod pom_xml;
mod project_py;
mod pyproject;
mod version_bump;
mod version_text;

pub use cargo_toml::CargoTomlEditor;
pub use cmake::CMakeListsEditor;
pub use homebrew::HomebrewFormulaEditor;
pub use package_json::PackageJsonEditor;
pub use pom_xml::PomXmlEditor;
pub use project_py::PythonVersionEditor;
pub use pyproject::PyprojectEditor;
pub use version_bump::{BumpType, EditorConfig, Version, apply_bump};
pub use version_text::VersionTextEditor;

use std::path::Path;

#[derive(Debug, thiserror::Error)]

pub enum EditorError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),

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
    fn name(&self) -> &'static str;
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
    pub parent_version: Option<VersionPosition>,
    pub is_workspace_root: bool,
    pub dependency_refs: Vec<DependencyRef>,
}

#[derive(Debug, Clone)]

pub struct VersionPosition {
    pub start: usize,
    pub end: usize,
    pub line: usize,
}

#[derive(Debug, Clone)]

pub struct DependencyRef {
    pub name_pattern: String,
    pub position: VersionPosition,
}

pub struct EditorRegistry {
    editors: std::collections::HashMap<&'static str, std::sync::Arc<dyn FileEditor>>,
    file_pattern_map: std::collections::HashMap<String, &'static str>,
}

impl EditorRegistry {
    pub fn new() -> Self {
        Self {
            editors: std::collections::HashMap::new(),
            file_pattern_map: std::collections::HashMap::new(),
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
        let name = editor.name();
        let patterns: Vec<String> = editor
            .file_patterns()
            .iter()
            .map(|s| s.to_string())
            .collect();

        let editor_arc: std::sync::Arc<dyn FileEditor> = std::sync::Arc::new(editor);

        for pattern in &patterns {
            self.file_pattern_map.insert(pattern.clone(), name);
        }

        self.editors.insert(name, editor_arc);
        self
    }

    pub fn get(&self, name: &str) -> Option<std::sync::Arc<dyn FileEditor>> {
        self.editors.get(name).cloned()
    }

    pub fn detect_editor(&self, path: &Path) -> Option<std::sync::Arc<dyn FileEditor>> {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        let parent_dir = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("");

        for (pattern, editor_name) in &self.file_pattern_map {
            if pattern.contains("{parent}") {
                let replaced = pattern.replace("{parent}", parent_dir);
                if file_name == replaced || path.ends_with(&replaced) {
                    return self.editors.get(editor_name).cloned();
                }
            } else if file_name == *pattern || path.ends_with(pattern) {
                return self.editors.get(editor_name).cloned();
            }
        }

        for editor in self.editors.values() {
            if editor.matches_file(path) {
                return Some(editor.clone());
            }
        }

        None
    }

    pub fn edit_version(
        &self,
        editor: &dyn FileEditor,
        content: &str,
        version: &str,
    ) -> Result<String> {
        let location = editor.parse(content)?;
        let edited = editor.edit(content, &location, version)?;
        editor.validate(content, &edited)?;
        Ok(edited)
    }

    pub fn edit_file(&self, path: &Path, new_version: &str) -> Result<()> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| EditorError::FileNotFound(format!("{}: {}", path.display(), e)))?;

        let editor = self
            .detect_editor(path)
            .ok_or_else(|| EditorError::UnsupportedFileType(format!("{}", path.display())))?;

        let edited = self.edit_version(&*editor, &content, new_version)?;

        write_with_backup(&path.to_string_lossy(), &edited)?;
        Ok(())
    }

    pub fn list(&self) -> Vec<&'static str> {
        self.editors.keys().copied().collect()
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

pub fn bump_version_in_file(
    path: &Path,
    bump_type: BumpType,
    config: &EditorConfig,
) -> Result<String> {
    let registry = EditorRegistry::default_with_editors();

    let content = std::fs::read_to_string(path)
        .map_err(|e| EditorError::FileNotFound(format!("{}: {}", path.display(), e)))?;

    let editor = registry
        .detect_editor(path)
        .ok_or_else(|| EditorError::UnsupportedFileType(format!("{}", path.display())))?;

    let location = editor.parse(&content)?;

    let current_version = if let Some(pos) = &location.project_version {
        content[pos.start..pos.end].to_string()
    } else {
        return Err(EditorError::VersionNotFound(format!(
            "No version found in {}",
            path.display()
        )));
    };

    let cleaned_version = current_version
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string();

    let new_version = apply_bump(&cleaned_version, &bump_type)?;

    if config.dry_run {
        return Ok(format!(
            "Would update {} from {} to {}",
            path.display(),
            cleaned_version,
            new_version
        ));
    }

    let edited = registry.edit_version(&*editor, &content, &new_version)?;

    write_with_backup(&path.to_string_lossy(), &edited)?;

    Ok(format!(
        "Updated {} from {} to {}",
        path.display(),
        cleaned_version,
        new_version
    ))
}

pub fn get_version_from_file(path: &Path) -> Result<String> {
    let registry = EditorRegistry::default_with_editors();

    let content = std::fs::read_to_string(path)
        .map_err(|e| EditorError::FileNotFound(format!("{}: {}", path.display(), e)))?;

    let editor = registry
        .detect_editor(path)
        .ok_or_else(|| EditorError::UnsupportedFileType(format!("{}", path.display())))?;

    let location = editor.parse(&content)?;

    if let Some(pos) = &location.project_version {
        let version = content[pos.start..pos.end].to_string();
        Ok(version
            .trim_matches('"')
            .trim_matches('\'')
            .trim()
            .to_string())
    } else {
        Err(EditorError::VersionNotFound(format!(
            "No version found in {}",
            path.display()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use file_types::{FileType, detect_file_type};

    #[test]
    fn test_detect_file_type() {
        assert_eq!(
            detect_file_type(Path::new("Cargo.toml")),
            Some(FileType::CargoToml)
        );
        assert_eq!(
            detect_file_type(Path::new("package.json")),
            Some(FileType::PackageJson)
        );
        assert_eq!(
            detect_file_type(Path::new("pyproject.toml")),
            Some(FileType::PyProject)
        );
        assert_eq!(
            detect_file_type(Path::new("CMakeLists.txt")),
            Some(FileType::Cmake)
        );
        assert_eq!(
            detect_file_type(Path::new("pom.xml")),
            Some(FileType::PomXml)
        );
        assert_eq!(
            detect_file_type(Path::new("version.txt")),
            Some(FileType::VersionText)
        );
        assert_eq!(
            detect_file_type(Path::new("test.rb")),
            Some(FileType::Homebrew)
        );
        assert_eq!(
            detect_file_type(Path::new("test.py")),
            Some(FileType::ProjectPy)
        );
        assert_eq!(detect_file_type(Path::new("unknown.txt")), None);
    }

    #[test]
    fn test_apply_bump() {
        assert_eq!(apply_bump("1.2.3", &BumpType::Major).unwrap(), "2.0.0");
        assert_eq!(apply_bump("1.2.3", &BumpType::Minor).unwrap(), "1.3.0");
        assert_eq!(apply_bump("1.2.3", &BumpType::Patch).unwrap(), "1.2.4");
        assert_eq!(
            apply_bump("1.2.3", &BumpType::PreRelease("beta".to_string())).unwrap(),
            "1.2.3-beta"
        );
        assert_eq!(
            apply_bump("1.2.3", &BumpType::Build("20240101".to_string())).unwrap(),
            "1.2.3+20240101"
        );

        // Test invalid versions
        assert!(apply_bump("invalid", &BumpType::Patch).is_err());
        assert!(apply_bump("1.2", &BumpType::Patch).is_err());
    }

    #[test]
    fn test_editor_registry_default() {
        let registry = EditorRegistry::default_with_editors();
        let editors = registry.list();

        assert!(editors.contains(&"cargo_toml"));
        assert!(editors.contains(&"package_json"));
        assert!(editors.contains(&"version_text"));
        assert!(editors.contains(&"cmake"));
        assert!(editors.contains(&"homebrew"));
        assert!(editors.contains(&"pom_xml"));
        assert!(editors.contains(&"project_py"));
        assert!(editors.contains(&"pyproject"));
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

        let edited_crlf = "line1\r\nline2\r\n";
        assert_eq!(
            preserve_line_endings(original_lf, edited_crlf.to_string()),
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
