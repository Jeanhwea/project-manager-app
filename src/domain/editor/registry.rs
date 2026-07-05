use super::cargo_toml::CargoTomlEditor;
use super::cmake::CMakeListsEditor;
use super::file_editor::FileEditor;
use super::homebrew::HomebrewFormulaEditor;
use super::package_json::PackageJsonEditor;
use super::pom_xml::PomXmlEditor;
use super::project_py::PythonVersionEditor;
use super::pyproject::PyprojectEditor;
use super::tauri_conf::TauriConfEditor;
use super::version_text::VersionTextEditor;
use std::path::Path;

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
            .register(TauriConfEditor)
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
