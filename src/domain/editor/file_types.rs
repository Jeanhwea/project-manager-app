//! File type detection utilities for the editor module

use std::path::Path;

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
        ("version.txt", _) => Some(FileType::VersionText),
        (_, "rb") => Some(FileType::Homebrew),
        (_, "py") => Some(FileType::ProjectPy),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_detect_file_type() {
        assert_eq!(
            detect_file_type(Path::new("Cargo.toml")),
            Some(FileType::CargoToml)
        );
        assert_eq!(
            detect_file_type(Path::new("package.json")),
            Some(FileType::PackageJson)
        );
        assert_eq!(
            detect_file_type(Path::new("pyproject.toml")),
            Some(FileType::PyProject)
        );
        assert_eq!(
            detect_file_type(Path::new("CMakeLists.txt")),
            Some(FileType::Cmake)
        );
        assert_eq!(
            detect_file_type(Path::new("pom.xml")),
            Some(FileType::PomXml)
        );
        assert_eq!(
            detect_file_type(Path::new("version.txt")),
            Some(FileType::VersionText)
        );
        assert_eq!(
            detect_file_type(Path::new("test.rb")),
            Some(FileType::Homebrew)
        );
        assert_eq!(
            detect_file_type(Path::new("test.py")),
            Some(FileType::ProjectPy)
        );
        assert_eq!(detect_file_type(Path::new("unknown.txt")), None);
    }
}