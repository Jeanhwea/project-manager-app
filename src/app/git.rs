use super::runner::CommandRunner;
use super::version::compare_versions;

pub fn get_current_version() -> Option<String> {
    let output = CommandRunner::run("git", &["tag", "-l", "v*"]).ok()?;

    let tags = String::from_utf8(output.stdout).ok()?;
    let mut tags: Vec<&str> = tags.lines().collect();

    if tags.is_empty() {
        return None;
    }

    tags.sort_by(|a, b| compare_versions(a, b));

    tags.first().map(|s| s.to_string())
}

pub fn create_tag(tag: &str) -> Result<(), String> {
    CommandRunner::run_with_success("git", &["tag", tag])?;
    CommandRunner::run_with_success("git", &["push", "origin", tag])?;
    Ok(())
}
