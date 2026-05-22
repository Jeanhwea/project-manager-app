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

pub use detect::{
    add_lockfile_operations, compute_edited_content, detect_config_files, read_file_version,
    resolve_config_files,
};
pub use error::{EditorError, Result};
pub use file_editor::{FileEditor, write_with_backup};
pub use position::{
    VersionLocation, VersionPosition, extract_version_position, replace_at_position,
};
pub use registry::EditorRegistry;
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
