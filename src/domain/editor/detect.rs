use super::{EditorRegistry, FileEditor};
use crate::domain::git::ReleaseError;
use crate::error::Result;
use std::path::Path;

pub fn resolve_config_files(registry: &EditorRegistry, files: &[String]) -> Result<Vec<String>> {
    if files.is_empty() {
        return detect_config_files(registry);
    }

    files
        .iter()
        .filter(|f| registry.detect_editor(Path::new(f)).is_some())
        .cloned()
        .map(Ok)
        .collect()
}

pub fn detect_config_files(registry: &EditorRegistry) -> Result<Vec<String>> {
    let mut result = Vec::new();

    for candidate in registry.candidate_files() {
        if candidate.contains("{}") {
            for path in expand_glob_pattern(candidate) {
                if Path::new(&path).exists() && registry.detect_editor(Path::new(&path)).is_some()
                {
                    result.push(path);
                }
            }
        } else if Path::new(candidate).exists()
            && registry.detect_editor(Path::new(candidate)).is_some()
        {
            result.push(candidate.to_string());
        }
    }

    if result.is_empty() {
        return Err(ReleaseError::NoConfigFiles.into());
    }

    Ok(result)
}

pub fn expand_glob_pattern(pattern: &str) -> Vec<String> {
    let mut results = Vec::new();
    let (prefix, suffix) = match pattern.split_once("{}") {
        Some(pair) => pair,
        None => return results,
    };

    let scan_dir = if prefix.is_empty() {
        ".".to_string()
    } else {
        prefix.trim_end_matches('/').to_string()
    };

    let entries = match std::fs::read_dir(&scan_dir) {
        Ok(e) => e,
        Err(_) => return results,
    };

    for entry in entries.flatten() {
        if entry.path().is_dir() {
            let dir_name = entry.file_name().to_string_lossy().to_string();
            if dir_name.starts_with('.') || dir_name == "node_modules" {
                continue;
            }
            results.push(format!("{}{}{}", prefix, dir_name, suffix));
        }
    }

    results
}

pub fn compute_edited_content(
    editor: &dyn FileEditor,
    tag: &str,
    config_file: &str,
) -> Result<(String, String)> {
    let version = tag.trim_start_matches('v');
    let content = std::fs::read_to_string(config_file).map_err(|e| ReleaseError::ReadFile {
        path: config_file.to_string(),
        source: e,
    })?;

    let location = editor.parse(&content)?;
    let edited = editor.edit(&content, &location, version)?;
    editor.validate(&content, &edited)?;

    Ok((content, edited))
}

pub fn read_file_version(editor: &dyn FileEditor, config_file: &str) -> Result<String> {
    let content = std::fs::read_to_string(config_file).map_err(|e| ReleaseError::ReadFile {
        path: config_file.to_string(),
        source: e,
    })?;
    let location = editor.parse(&content)?;
    let pos =
        location
            .project_version
            .as_ref()
            .ok_or_else(|| ReleaseError::VersionFieldNotFound {
                path: config_file.to_string(),
            })?;
    let version_str = &content[pos.start..pos.end];
    Ok(version_str.to_string())
}

pub fn extract_fallback_version(
    registry: &EditorRegistry,
    config_files: &[String],
) -> Option<String> {
    use super::Version;

    let mut best: Option<Version> = None;
    for file_path in config_files {
        let editor = registry.detect_editor(Path::new(file_path))?;
        if let Ok(ver_str) = read_file_version(editor, file_path)
            && let Ok(ver) = Version::parse(&ver_str)
            && best.as_ref().is_none_or(|b| ver > *b)
        {
            best = Some(ver);
        }
    }
    best.map(|v| v.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::editor::lockfile::read_cargo_package_name;
    use tempfile::tempdir;

    #[test]
    fn test_expand_glob_pattern() {
        let temp_dir = tempdir().unwrap();
        let dir1 = temp_dir.path().join("dir1");
        let dir2 = temp_dir.path().join("dir2");
        std::fs::create_dir(&dir1).unwrap();
        std::fs::create_dir(&dir2).unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let expanded = expand_glob_pattern("{}/package.json");

        assert!(expanded.contains(&"dir1/package.json".to_string()));
        assert!(expanded.contains(&"dir2/package.json".to_string()));

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_read_cargo_package_name() {
        let temp_dir = tempdir().unwrap();
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");

        std::fs::write(
            &cargo_toml_path,
            r#"[package]
name = "test-package"
version = "0.1.0"

[dependencies]
serde = "1.0""#,
        )
        .unwrap();

        let package_name = read_cargo_package_name(&cargo_toml_path.to_string_lossy()).unwrap();
        assert_eq!(package_name, "test-package");
    }

    #[test]
    fn test_read_cargo_package_name_not_found() {
        let temp_dir = tempdir().unwrap();
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");

        std::fs::write(
            &cargo_toml_path,
            r#"[dependencies]
serde = "1.0""#,
        )
        .unwrap();

        assert!(read_cargo_package_name(&cargo_toml_path.to_string_lossy()).is_err());
    }
}
