//! 编辑器注册表：按文件名把路径分发到对应的 FileEditor 实现。
//! 注册顺序即匹配优先级，detect_editor 取第一个 matches_file 为真的编辑器。

use super::app_json::AppJsonEditor;
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

/// 已注册编辑器的集合。
pub struct EditorRegistry {
    editors: Vec<Box<dyn FileEditor>>,
}

impl std::fmt::Debug for EditorRegistry {
    // dyn FileEditor 无法 Debug，只输出数量
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EditorRegistry")
            .field("editors", &self.editors.len())
            .finish()
    }
}

impl EditorRegistry {
    /// 不含任何编辑器的空注册表。
    pub fn new() -> Self {
        Self {
            editors: Vec::new(),
        }
    }

    /// 注册全部内置编辑器，顺序决定了文件名的匹配优先级。
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
            .register(AppJsonEditor)
    }

    /// 追加一个编辑器并返回自身，便于链式注册。
    pub fn register(mut self, editor: impl FileEditor + 'static) -> Self {
        self.editors.push(Box::new(editor));
        self
    }

    /// 找出能处理该路径的编辑器，没有任何编辑器匹配时返回 None。
    pub fn detect_editor(&self, path: &Path) -> Option<&dyn FileEditor> {
        self.editors
            .iter()
            .find(|editor| editor.matches_file(path))
            .map(|e| e.as_ref())
    }

    /// 汇总所有编辑器的候选路径，供自动探测时逐个检查是否存在。
    pub fn candidate_files(&self) -> Vec<&str> {
        self.editors
            .iter()
            .flat_map(|editor| editor.candidate_files())
            .collect()
    }
}

impl Default for EditorRegistry {
    /// 默认即内置全部编辑器。
    fn default() -> Self {
        Self::default_with_editors()
    }
}
