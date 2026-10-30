//! 外部命令可用性探测。

use std::process::{Command, Stdio};

/// 判断命令是否在 PATH 中可用
// 忽略命令自身的 stdout/stderr，探测过程不应污染 pma 的输出
pub fn is_command_available(name: &str) -> bool {
    // Windows 上不少命令没有 --version 或会以非零退出，失败后再用 where 兜一次
    #[cfg(target_os = "windows")]
    {
        let direct_result = Command::new(name)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());

        if direct_result {
            true
        } else {
            Command::new("where")
                .arg(name)
                .output()
                .is_ok_and(|output| output.status.success())
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new(name)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }
}
