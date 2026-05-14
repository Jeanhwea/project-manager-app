pub mod output;
pub mod path;

pub fn is_command_available(name: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        let direct_result = std::process::Command::new(name)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success());

        if direct_result {
            true
        } else {
            std::process::Command::new("where")
                .arg(name)
                .output()
                .is_ok_and(|output| output.status.success())
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new(name)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }
}
