pub mod output;
pub mod path;
pub mod process;

pub use process::is_command_available;

pub fn format_duration(secs: u64) -> String {
    if secs >= 60 && secs.is_multiple_of(60) {
        format!("{} 分钟", secs / 60)
    } else {
        format!("{} 秒", secs)
    }
}
