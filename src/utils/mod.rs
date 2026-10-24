pub mod output;
pub mod path;
pub mod process;

pub use process::is_command_available;

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
