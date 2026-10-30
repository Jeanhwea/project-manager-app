//! 通用工具集合：终端输出（`output`）、路径规整（`path`）、外部命令探测（`process`），
//! 以及一些与具体模块无关的小函数。

pub mod output;
pub mod path;
pub mod process;

pub use process::is_command_available;

/// 把秒数渲染成中文可读时长，不足 1 分钟时只显示秒
pub fn format_duration(secs: u64) -> String {
    if secs >= 60 {
        let minutes = secs / 60;
        let rest = secs % 60;
        if rest == 0 {
            format!("{} 分钟", minutes)
        } else {
            format!("{} 分 {} 秒", minutes, rest)
        }
    } else {
        format!("{} 秒", secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(0), "0 秒");
        assert_eq!(format_duration(45), "45 秒");
    }

    #[test]
    fn test_format_duration_exact_minutes() {
        assert_eq!(format_duration(60), "1 分钟");
        assert_eq!(format_duration(30 * 60), "30 分钟");
    }

    #[test]
    fn test_format_duration_minutes_and_seconds() {
        assert_eq!(format_duration(90), "1 分 30 秒");
        assert_eq!(format_duration(3590), "59 分 50 秒");
    }
}
