use std::path::Path;

/// 优化路径显示，移除 Windows UNC 路径前缀
pub fn format_path(path: &Path) -> String {
    path.to_string_lossy()
        .trim_start_matches("\\\\?\\")
        .to_string()
}

pub fn get_current_dir() -> String {
    std::env::current_dir()
        .expect("Failed to get current directory")
        .canonicalize()
        .unwrap_or_else(|e| {
            eprintln!("错误: {}", e);
            std::process::exit(1);
        })
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string()
}
