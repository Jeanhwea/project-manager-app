use super::version::compare_versions;
use std::process::Command;

pub fn get_current_version() -> Option<String> {
    let output = Command::new("git")
        .args(["tag", "-l", "v*"])
        .output()
        .ok()?;

    let tags = String::from_utf8(output.stdout).ok()?;
    let mut tags: Vec<&str> = tags.lines().collect();

    if tags.is_empty() {
        return None;
    }

    tags.sort_by(|a, b| compare_versions(a, b));

    tags.first().map(|s| s.to_string())
}

pub fn create_tag(tag: &str) -> Result<(), String> {
    let output = Command::new("git")
        .args(["tag", tag])
        .output()
        .map_err(|e| format!("执行 git tag 失败: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("创建 tag 失败: {}", stderr));
    }

    let output = Command::new("git")
        .args(["push", "origin", tag])
        .output()
        .map_err(|e| format!("执行 git push 失败: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("推送 tag 失败: {}", stderr));
    }

    Ok(())
}
