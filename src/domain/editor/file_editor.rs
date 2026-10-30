//! FileEditor trait：所有版本号编辑器统一实现的接口，以及落盘用的辅助函数。
//! 默认实现（parse / edit / validate）适用于「定位一次版本号再做纯文本替换」的常见情况，
//! 格式特殊的编辑器（如 Cargo.toml、pyproject.toml）可自行覆盖。

use super::error::{EditorError, Result};
use super::position::{VersionLocation, VersionPosition, replace_at_position};
use std::path::Path;

/// 一种配置文件的版本号编辑器，要求可跨线程共享。
pub trait FileEditor: Send + Sync {
    /// 编辑器的显示名，用于日志与错误提示。
    fn name(&self) -> &str;
    /// 该编辑器负责的文件名模式，可含 `{}` / `{parent}` 占位符。
    fn file_patterns(&self) -> &[&str];
    /// 在文件内容中定位版本号的字节范围，找不到返回 None。
    fn find_version(&self, content: &str) -> Option<VersionPosition>;

    /// 自动探测时要去磁盘上找的候选路径，默认等于 file_patterns()。
    fn candidate_files(&self) -> Vec<&str> {
        self.file_patterns().to_vec()
    }

    fn matches_file(&self, path: &Path) -> bool {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        self.file_patterns().iter().any(|pattern| {
            // 占位符语义：{parent} 要求文件名与父目录同名，{} 匹配任意父目录名，
            // 前者形如 foo.rb，后者形如 <任意目录>/__version__.py
            if pattern.contains("{parent}") {
                let parent_dir = path
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                let replaced = pattern.replace("{parent}", parent_dir);
                file_name == replaced || path.ends_with(&replaced)
            } else if pattern.contains("{}") {
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

    /// 解析文件内容并给出版本号位置；没有版本字段时返回 VersionNotFound。
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

    /// 按已定位的范围替换版本号，返回改写后的完整内容。
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

    /// 校验改写结果是否仍是合法的原格式，默认不检查。
    fn validate(&self, _original: &str, _edited: &str) -> Result<()> {
        Ok(())
    }
}

/// 写文件：先写 `.tmp` 再 rename 覆盖目标，避免目标文件被写成半截内容。
pub fn write_with_backup(path: &str, content: &str) -> Result<()> {
    let tmp_path = format!("{}.tmp", path);
    std::fs::write(&tmp_path, content).map_err(EditorError::WriteError)?;
    std::fs::rename(&tmp_path, path).map_err(|e| {
        // rename 失败时清理临时文件，避免在工作区留下垃圾
        let _ = std::fs::remove_file(&tmp_path);
        EditorError::WriteError(e)
    })
}
