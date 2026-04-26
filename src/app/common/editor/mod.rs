mod cargo_toml;
mod cmake;
mod homebrew;
mod package_json;
mod pom_xml;
mod project_py;
mod pyproject;
mod version_text;

pub use cargo_toml::CargoTomlEditor;
pub use cmake::CMakeListsEditor;
pub use homebrew::HomebrewFormulaEditor;
pub use package_json::PackageJsonEditor;
pub use pom_xml::PomXmlEditor;
pub use project_py::PythonVersionEditor;
pub use pyproject::PyprojectEditor;
pub use version_text::VersionTextEditor;

pub trait ConfigEditor {
    fn parse(&self, content: &str) -> Result<VersionLocation, VersionEditError>;
    fn edit(
        &self,
        content: &str,
        location: &VersionLocation,
        new_version: &str,
    ) -> Result<String, VersionEditError>;
    fn validate(&self, original: &str, edited: &str) -> Result<(), VersionEditError>;
}

pub struct VersionLocation {
    pub project_version: Option<VersionPosition>,
    pub parent_version: Option<VersionPosition>,
    pub is_workspace_root: bool,
    pub dependency_refs: Vec<DependencyRef>,
}

pub struct VersionPosition {
    pub start: usize,
    pub end: usize,
    #[allow(dead_code)]
    pub line: usize,
}

pub struct DependencyRef {
    pub name_pattern: String,
    #[allow(dead_code)]
    pub position: VersionPosition,
}

#[derive(Debug)]
pub enum VersionEditError {
    #[allow(dead_code)]
    FileNotFound(String),
    ParseError {
        file: String,
        reason: String,
    },
    VersionNotFound {
        file: String,
        hint: String,
    },
    WriteError {
        file: String,
        reason: String,
    },
    FormatPreservationError {
        file: String,
    },
}

impl std::fmt::Display for VersionEditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionEditError::FileNotFound(file) => {
                write!(f, "文件不存在: {}", file)
            }
            VersionEditError::ParseError { file, reason } => {
                write!(
                    f,
                    "解析 {} 失败: {}。请检查文件格式是否正确。",
                    file, reason
                )
            }
            VersionEditError::VersionNotFound { file, hint } => {
                write!(f, "{} 未找到版本字段。{}", file, hint)
            }
            VersionEditError::WriteError { file, reason } => {
                write!(f, "写入 {} 失败: {}。已从备份恢复原文件。", file, reason)
            }
            VersionEditError::FormatPreservationError { file } => {
                write!(f, "编辑 {} 后格式验证失败，已取消修改。", file)
            }
        }
    }
}

impl std::error::Error for VersionEditError {}

impl From<std::io::Error> for VersionEditError {
    fn from(err: std::io::Error) -> Self {
        VersionEditError::WriteError {
            file: String::new(),
            reason: err.to_string(),
        }
    }
}

pub fn write_with_backup(path: &str, content: &str) -> Result<(), VersionEditError> {
    let backup_path = format!("{}.bak", path);

    std::fs::copy(path, &backup_path).map_err(|e| VersionEditError::WriteError {
        file: path.to_string(),
        reason: format!("无法创建备份: {}", e),
    })?;

    match std::fs::write(path, content) {
        Ok(_) => {
            let _ = std::fs::remove_file(&backup_path);
            Ok(())
        }
        Err(e) => {
            let restore_result = std::fs::rename(&backup_path, path);
            Err(VersionEditError::WriteError {
                file: path.to_string(),
                reason: if restore_result.is_ok() {
                    format!("{} (已从备份恢复)", e)
                } else {
                    format!("{} (备份恢复也失败)", e)
                },
            })
        }
    }
}
