//! File system operation utilities
//!
//! This module provides cross-platform file system operations with consistent
//! error handling and context. All functions return `std::io::Result` with
//! descriptive error messages.

use std::fs;
use std::io;
use std::path::Path;

/// Read file contents as string with error context
pub fn read_to_string(path: impl AsRef<Path>) -> io::Result<String> {
    let path = path.as_ref();
    fs::read_to_string(path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("Failed to read file '{}': {}", path.display(), e),
        )
    })
}

/// Write string to file with error context
pub fn write_string(path: impl AsRef<Path>, contents: &str) -> io::Result<()> {
    let path = path.as_ref();
    fs::write(path, contents).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("Failed to write file '{}': {}", path.display(), e),
        )
    })
}

/// Copy file from source to destination with error context
pub fn copy_file(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<u64> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    fs::copy(src, dst).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!(
                "Failed to copy file '{}' to '{}': {}",
                src.display(),
                dst.display(),
                e
            ),
        )
    })
}

/// Rename or move file with error context
pub fn rename_file(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    fs::rename(src, dst).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!(
                "Failed to rename file '{}' to '{}': {}",
                src.display(),
                dst.display(),
                e
            ),
        )
    })
}

/// Check if a file exists
pub fn file_exists(path: impl AsRef<Path>) -> bool {
    path.as_ref().is_file()
}

/// Check if a directory exists
pub fn dir_exists(path: impl AsRef<Path>) -> bool {
    path.as_ref().is_dir()
}

/// Check if a path exists (file or directory)
pub fn path_exists(path: impl AsRef<Path>) -> bool {
    path.as_ref().exists()
}

/// Get file metadata with error context
pub fn metadata(path: impl AsRef<Path>) -> io::Result<fs::Metadata> {
    let path = path.as_ref();
    fs::metadata(path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("Failed to get metadata for '{}': {}", path.display(), e),
        )
    })
}

/// Get file size in bytes with error context
pub fn file_size(path: impl AsRef<Path>) -> io::Result<u64> {
    metadata(path).map(|m| m.len())
}

/// Check if path is a symbolic link
pub fn is_symlink(path: impl AsRef<Path>) -> bool {
    path.as_ref().is_symlink()
}

/// Create directory recursively with error context
pub fn create_dir_all(path: impl AsRef<Path>) -> io::Result<()> {
    let path = path.as_ref();
    fs::create_dir_all(path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("Failed to create directory '{}': {}", path.display(), e),
        )
    })
}

/// Create directory (non-recursive) with error context
pub fn create_dir(path: impl AsRef<Path>) -> io::Result<()> {
    let path = path.as_ref();
    fs::create_dir(path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("Failed to create directory '{}': {}", path.display(), e),
        )
    })
}

/// Remove file with error context
pub fn remove_file(path: impl AsRef<Path>) -> io::Result<()> {
    let path = path.as_ref();
    fs::remove_file(path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("Failed to remove file '{}': {}", path.display(), e),
        )
    })
}

/// Remove directory (non-recursive) with error context
pub fn remove_dir(path: impl AsRef<Path>) -> io::Result<()> {
    let path = path.as_ref();
    fs::remove_dir(path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("Failed to remove directory '{}': {}", path.display(), e),
        )
    })
}

/// Remove directory recursively with error context
pub fn remove_dir_all(path: impl AsRef<Path>) -> io::Result<()> {
    let path = path.as_ref();
    fs::remove_dir_all(path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("Failed to remove directory '{}': {}", path.display(), e),
        )
    })
}

/// Read directory entries with error context
pub fn read_dir(path: impl AsRef<Path>) -> io::Result<fs::ReadDir> {
    let path = path.as_ref();
    fs::read_dir(path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("Failed to read directory '{}': {}", path.display(), e),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_file_operations() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Test write and read
        let content = "Hello, world!";
        write_string(&file_path, content).unwrap();
        assert!(file_exists(&file_path));
        assert!(path_exists(&file_path));

        let read_content = read_to_string(&file_path).unwrap();
        assert_eq!(read_content, content);

        // Test file size
        let size = file_size(&file_path).unwrap();
        assert_eq!(size, content.len() as u64);

        // Test metadata
        let metadata = metadata(&file_path).unwrap();
        assert!(metadata.is_file());

        // Test copy
        let copy_path = temp_dir.path().join("copy.txt");
        copy_file(&file_path, &copy_path).unwrap();
        assert!(file_exists(&copy_path));

        // Test rename
        let renamed_path = temp_dir.path().join("renamed.txt");
        rename_file(&copy_path, &renamed_path).unwrap();
        assert!(!file_exists(&copy_path));
        assert!(file_exists(&renamed_path));

        // Test remove
        remove_file(&file_path).unwrap();
        assert!(!file_exists(&file_path));
    }

    #[test]
    fn test_dir_operations() {
        let temp_dir = tempdir().unwrap();
        let nested_dir = temp_dir.path().join("a").join("b").join("c");

        // Test create directory
        create_dir_all(&nested_dir).unwrap();
        assert!(dir_exists(&nested_dir));
        assert!(path_exists(&nested_dir));

        // Test create non-recursive directory
        let simple_dir = temp_dir.path().join("simple");
        create_dir(&simple_dir).unwrap();
        assert!(dir_exists(&simple_dir));

        // Test read directory
        let entries: Vec<_> = read_dir(temp_dir.path()).unwrap().collect();
        assert!(!entries.is_empty());

        // Test remove directory (non-recursive)
        remove_dir(&simple_dir).unwrap();
        assert!(!dir_exists(&simple_dir));

        // Test remove directory recursively
        remove_dir_all(temp_dir.path().join("a")).unwrap();
        assert!(!dir_exists(&nested_dir));
    }

    #[test]
    fn test_error_handling() {
        // Test reading non-existent file
        let result = read_to_string("/nonexistent/file.txt");
        assert!(result.is_err());

        // Test writing to invalid path
        let result = write_string("/invalid/\0/path.txt", "test");
        assert!(result.is_err());

        // Test copying non-existent file
        let result = copy_file("/nonexistent/src.txt", "/nonexistent/dst.txt");
        assert!(result.is_err());

        // Test removing non-existent file
        let result = remove_file("/nonexistent/file.txt");
        assert!(result.is_err());
    }
}
