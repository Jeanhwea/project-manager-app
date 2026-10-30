//! tauri.conf.json 编辑器。
//! 版本号定位策略：匹配第一次出现的 `"version": "..."`（Tauri 配置文件里它是顶层字段）；
//! 改写走纯文本替换，以保留原有 JSON 排版与键顺序。

use super::{EditorError, FileEditor, Result, VersionPosition, extract_version_position};

/// 处理 tauri.conf.json 的顶层 version 字段。
pub struct TauriConfEditor;

impl TauriConfEditor {
    fn find_version_position(content: &str) -> Option<VersionPosition> {
        let pattern = regex::Regex::new(r#""version"\s*:\s*"([^"]*)""#).ok()?;
        extract_version_position(content, &pattern)
    }
}

impl FileEditor for TauriConfEditor {
    fn name(&self) -> &str {
        "tauri.conf.json"
    }

    fn file_patterns(&self) -> &[&str] {
        &["tauri.conf.json"]
    }

    fn candidate_files(&self) -> Vec<&str> {
        vec!["tauri.conf.json", "src-tauri/tauri.conf.json"]
    }

    fn find_version(&self, content: &str) -> Option<VersionPosition> {
        Self::find_version_position(content)
    }

    fn parse(&self, content: &str) -> Result<super::VersionLocation> {
        // 只做 JSON 合法性校验，不参与改写：重新序列化会丢失原有排版
        let _: serde_json::Value = serde_json::from_str(content).map_err(|e| {
            EditorError::ParseError(format!("Failed to parse tauri.conf.json: {}", e))
        })?;

        let project_version = self.find_version(content);

        if project_version.is_none() {
            return Err(EditorError::VersionNotFound(
                "tauri.conf.json does not have version field".to_string(),
            ));
        }

        Ok(super::VersionLocation {
            project_version,
            is_workspace_root: false,
        })
    }

    fn validate(&self, original: &str, edited: &str) -> Result<()> {
        if serde_json::from_str::<serde_json::Value>(edited).is_err() {
            return Err(EditorError::FormatPreservationError(
                "tauri.conf.json format validation failed".to_string(),
            ));
        }

        // 纯文本替换只应带来版本号长度的变化，长度突变说明定位错了
        let original_len = original.len();
        let edited_len = edited.len();
        if edited_len.abs_diff(original_len) > original_len / 2 {
            return Err(EditorError::FormatPreservationError(
                "tauri.conf.json changed too much".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tauri_conf_editor_basic() {
        let content = r#"{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "shared-space-app",
  "version": "0.4.8",
  "identifier": "io.github.jeanhwea.shared-space-app"
}"#;

        let editor = TauriConfEditor;
        let location = editor.parse(content).unwrap();
        assert!(location.project_version.is_some());

        let edited = editor.edit(content, &location, "0.5.0").unwrap();
        assert!(edited.contains("\"version\": \"0.5.0\""));
        assert!(!edited.contains("\"version\": \"0.4.8\""));
        assert!(edited.contains("\"productName\": \"shared-space-app\""));
        editor.validate(content, &edited).unwrap();
    }

    #[test]
    fn test_tauri_conf_editor_no_version() {
        let content = r#"{
  "productName": "app"
}"#;
        let editor = TauriConfEditor;
        assert!(editor.parse(content).is_err());
    }

    #[test]
    fn test_tauri_conf_editor_invalid_json() {
        let content = "{ not json";
        let editor = TauriConfEditor;
        assert!(editor.parse(content).is_err());
    }
}
