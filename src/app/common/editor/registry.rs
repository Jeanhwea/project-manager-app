use super::{ConfigEditor, VersionLocation};
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

pub struct EditorRegistry {
    editors: HashMap<&'static str, Arc<dyn ConfigEditor>>,
    file_pattern_map: HashMap<String, &'static str>,
}

impl EditorRegistry {
    pub fn new() -> Self {
        Self {
            editors: HashMap::new(),
            file_pattern_map: HashMap::new(),
        }
    }

    pub fn register(mut self, editor: impl ConfigEditor + 'static) -> Self {
        let name = editor.name();
        let patterns: Vec<String> = editor
            .file_patterns()
            .iter()
            .map(|s| s.to_string())
            .collect();

        let editor_arc: Arc<dyn ConfigEditor> = Arc::new(editor);

        for pattern in &patterns {
            self.file_pattern_map.insert(pattern.clone(), name);
        }

        self.editors.insert(name, editor_arc);
        self
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn ConfigEditor>> {
        self.editors.get(name).cloned()
    }

    pub fn detect_editor(&self, path: &Path) -> Option<Arc<dyn ConfigEditor>> {
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
        editor: &dyn ConfigEditor,
        content: &str,
        version: &str,
    ) -> Result<String> {
        let location = editor.parse(content).map_err(|e| anyhow!("{}", e))?;

        let edited = editor
            .edit(content, &location, version)
            .map_err(|e| anyhow!("{}", e))?;

        editor
            .validate(content, &edited)
            .map_err(|e| anyhow!("{}", e))?;

        Ok(edited)
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

impl VersionLocation {
    pub fn new() -> Self {
        Self {
            project_version: None,
            parent_version: None,
            is_workspace_root: false,
            dependency_refs: Vec::new(),
        }
    }

    pub fn with_version(mut self, start: usize, end: usize, line: usize) -> Self {
        self.project_version = Some(super::VersionPosition { start, end, line });
        self
    }
}

impl Default for VersionLocation {
    fn default() -> Self {
        Self::new()
    }
}
