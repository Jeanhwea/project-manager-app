//! Editor domain module
//!
//! This module contains file editing utilities for version bumping.

use std::path::Path;

/// Editor-specific error type
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

/// File types supported for version editing
#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    CargoToml,
    PackageJson,
    PyProject,
    VersionText,
    Cmake,
    PomXml,
    Homebrew,
    ProjectPy,
}

/// Version bump type
#[derive(Debug, Clone, PartialEq)]
pub enum BumpType {
    Major,
    Minor,
    Patch,
    PreRelease(String),
    Build(String),
}

/// Version editing configuration
#[derive(Debug, Clone)]
pub struct EditorConfig {
    pub dry_run: bool,
    pub skip_push: bool,
    pub force: bool,
    pub message: Option<String>,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            dry_run: false,
            skip_push: false,
            force: false,
            message: None,
        }
    }
}

/// Common result type for editor operations
pub type Result<T> = std::result::Result<T, EditorError>;

/// Trait for file editors that can modify version information
pub trait FileEditor: Send + Sync {
    /// Get the name of this editor
    fn name(&self) -> &'static str;
    
    /// Get file patterns this editor can handle
    fn file_patterns(&self) -> &[&str];
    
    /// Check if this editor can handle the given file
    fn matches_file(&self, path: &Path) -> bool;
    
    /// Parse the file content to find version information
    fn parse(&self, content: &str) -> Result<VersionLocation>;
    
    /// Edit the file content to update version
    fn edit(&self, content: &str, location: &VersionLocation, new_version: &str) -> Result<String>;
    
    /// Validate the edited content
    fn validate(&self, original: &str, edited: &str) -> Result<()>;
}

/// Location of version information within a file
#[derive(Debug, Clone)]
pub struct VersionLocation {
    /// Position of the main project version
    pub project_version: Option<VersionPosition>,
    /// Position of parent version (for workspace files)
    pub parent_version: Option<VersionPosition>,
    /// Whether this is a workspace root file
    pub is_workspace_root: bool,
    /// References to dependency versions that might need updating
    pub dependency_refs: Vec<DependencyRef>,
}

impl Default for VersionLocation {
    fn default() -> Self {
        Self {
            project_version: None,
            parent_version: None,
            is_workspace_root: false,
            dependency_refs: Vec::new(),
        }
    }
}

/// Position of version information within a file
#[derive(Debug, Clone)]
pub struct VersionPosition {
    /// Start byte offset
    pub start: usize,
    /// End byte offset
    pub end: usize,
    /// Line number (1-indexed)
    pub line: usize,
}

/// Reference to a dependency version that might need updating
#[derive(Debug, Clone)]
pub struct DependencyRef {
    /// Pattern to match dependency name
    pub name_pattern: String,
    /// Position of the version
    pub position: VersionPosition,
}

/// Registry for managing multiple file editors
pub struct EditorRegistry {
    editors: std::collections::HashMap<&'static str, std::sync::Arc<dyn FileEditor>>,
    file_pattern_map: std::collections::HashMap<String, &'static str>,
}

impl EditorRegistry {
    /// Create a new empty editor registry
    pub fn new() -> Self {
        Self {
            editors: std::collections::HashMap::new(),
            file_pattern_map: std::collections::HashMap::new(),
        }
    }
    
    /// Register an editor with the registry
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
    
    /// Get an editor by name
    pub fn get(&self, name: &str) -> Option<std::sync::Arc<dyn FileEditor>> {
        self.editors.get(name).cloned()
    }
    
    /// Detect which editor can handle the given file
    pub fn detect_editor(&self, path: &Path) -> Option<std::sync::Arc<dyn FileEditor>> {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        
        let parent_dir = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("");
        
        // Check file patterns
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
        
        // Fall back to editor-specific matching
        for editor in self.editors.values() {
            if editor.matches_file(path) {
                return Some(editor.clone());
            }
        }
        
        None
    }
    
    /// Edit version using the specified editor
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
    
    /// List all registered editor names
    pub fn list(&self) -> Vec<&'static str> {
        self.editors.keys().copied().collect()
    }
}

impl Default for EditorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility function to write file with backup
pub fn write_with_backup(path: &str, content: &str) -> Result<()> {
    let backup_path = format!("{}.bak", path);
    
    std::fs::copy(path, &backup_path).map_err(|e| EditorError::WriteError(e))?;
    
    match std::fs::write(path, content) {
        Ok(_) => {
            let _ = std::fs::remove_file(&backup_path);
            Ok(())
        }
        Err(e) => {
            let restore_result = std::fs::rename(&backup_path, path);
            Err(EditorError::WriteError(e))
        }
    }
}

/// Utility function to preserve line endings
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

/// Detect file type from path
pub fn detect_file_type(path: &Path) -> Option<FileType> {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    
    match (file_name, extension) {
        ("Cargo.toml", _) => Some(FileType::CargoToml),
        ("package.json", _) => Some(FileType::PackageJson),
        ("pyproject.toml", _) => Some(FileType::PyProject),
        ("CMakeLists.txt", _) => Some(FileType::Cmake),
        ("pom.xml", _) => Some(FileType::PomXml),
        (_, "txt") => Some(FileType::VersionText),
        (_, "rb") => Some(FileType::Homebrew),
        (_, "py") => Some(FileType::ProjectPy),
        _ => None,
    }
}

/// Apply version bump to a version string
pub fn apply_bump(version: &str, bump_type: &BumpType) -> Result<String> {
    // Parse semantic version
    let mut parts = version.split('.');
    let major = parts.next().and_then(|s| s.parse::<u32>().ok());
    let minor = parts.next().and_then(|s| s.parse::<u32>().ok());
    let patch = parts.next().and_then(|s| s.parse::<u32>().ok());
    
    match (major, minor, patch) {
        (Some(major), Some(minor), Some(patch)) => {
            let (new_major, new_minor, new_patch) = match bump_type {
                BumpType::Major => (major + 1, 0, 0),
                BumpType::Minor => (major, minor + 1, 0),
                BumpType::Patch => (major, minor, patch + 1),
                BumpType::PreRelease(label) => {
                    return Ok(format!("{}.{}.{}-{}", major, minor, patch, label))
                }
                BumpType::Build(label) => {
                    return Ok(format!("{}.{}.{}+{}", major, minor, patch, label))
                }
            };
            Ok(format!("{}.{}.{}", new_major, new_minor, new_patch))
        }
        _ => Err(EditorError::VersionFormatError(format!(
            "Invalid version format: {}",
            version
        ))),
    }
}