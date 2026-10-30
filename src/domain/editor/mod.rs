//! 版本号编辑器：把 release 流程里的「改版本号」抽象成一组可插拔的编辑器。
//! 每个编辑器负责一种配置文件（Cargo.toml、package.json、pyproject.toml 等），
//! 先在原文中定位版本号的字节范围，再按范围做纯文本替换，以尽量保留原文件格式。

mod app_json;
mod cargo_toml;
mod cmake;
mod detect;
mod error;
mod file_editor;
mod homebrew;
mod package_json;
mod pom_xml;
mod position;
mod project_py;
mod pyproject;
mod registry;
mod tauri_conf;
mod version_bump;
mod version_text;

/// 配置文件的发现、读取与改写编排入口。
pub use detect::{
    add_lockfile_operations, compute_edited_content, detect_config_files,
    extract_fallback_version, read_file_version, resolve_config_files,
};
/// 编辑器模块的错误类型与 Result 别名。
pub use error::{EditorError, Result};
/// FileEditor trait 与写文件的辅助函数。
pub use file_editor::{FileEditor, write_with_backup};
/// 版本号位置描述与按位置替换的工具函数。
pub use position::{
    VersionLocation, VersionPosition, extract_version_position, replace_at_position,
};
/// 按文件名把路径分发到具体编辑器的注册表。
pub use registry::EditorRegistry;
/// 版本号解析与递增。
pub use version_bump::{BumpType, Version};

#[cfg(test)]
mod tests {
    use super::*;
    use cargo_toml::CargoTomlEditor;
    use package_json::PackageJsonEditor;
    use std::path::Path;

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
    fn test_cargo_toml_workspace_package_version() {
        let content = r#"[workspace]
resolver = "2"
members = ["crates/foo"]

[workspace.package]
version = "0.1.5"
edition = "2024"

[workspace.dependencies]
serde = "1"
"#;

        let editor = CargoTomlEditor;
        let location = editor.parse(content).unwrap();

        assert!(location.project_version.is_some());
        assert!(!location.is_workspace_root);

        let edited = editor.edit(content, &location, "0.2.0").unwrap();
        assert!(edited.contains("version = \"0.2.0\""));
        assert!(!edited.contains("version = \"0.1.5\""));
    }

    #[test]
    fn test_cargo_toml_workspace_root_without_package_version() {
        let content = r#"[workspace]
resolver = "2"
members = ["crates/foo"]
"#;

        let editor = CargoTomlEditor;
        let location = editor.parse(content).unwrap();

        assert!(location.is_workspace_root);
        assert!(location.project_version.is_none());
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
}
