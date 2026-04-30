//! File system operation utilities

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

/// Check if a file exists
pub fn file_exists(path: impl AsRef<Path>) -> bool {
    path.as_ref().is_file()
}

/// Check if a directory exists
pub fn dir_exists(path: impl AsRef<Path>) -> bool {
    path.as_ref().is_dir()
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
        
        let read_content = read_to_string(&file_path).unwrap();
        assert_eq!(read_content, content);
        
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
        
        // Test remove directory
        remove_dir_all(temp_dir.path().join("a")).unwrap();
        assert!(!dir_exists(&nested_dir));
    }
}