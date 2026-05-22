use std::process::{Command, Stdio};

pub fn is_command_available(name: &str) -> bool {
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
