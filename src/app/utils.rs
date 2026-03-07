use std::path::Path;

/// 优化路径显示，移除 Windows UNC 路径前缀
pub fn format_path(path: &Path) -> String {
    let mut display_path = path.to_string_lossy().to_string();
    display_path = display_path.trim_start_matches("\\\\?\\").to_string();
    display_path
}
