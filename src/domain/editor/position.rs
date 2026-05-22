#[derive(Debug, Clone, Default)]
pub struct VersionLocation {
    pub project_version: Option<VersionPosition>,
    pub is_workspace_root: bool,
}

#[derive(Debug, Clone)]
pub struct VersionPosition {
    pub start: usize,
    pub end: usize,
}

pub fn replace_at_position(content: &str, pos: &VersionPosition, new_value: &str) -> String {
    let mut result = String::with_capacity(content.len() + new_value.len());
    result.push_str(&content[..pos.start]);
    result.push_str(new_value);
    result.push_str(&content[pos.end..]);
    result
}

pub fn extract_version_position(
    content: &str,
    pattern: &regex::Regex,
) -> Option<VersionPosition> {
    let caps = pattern.captures(content)?;
    let version_match = caps.get(1)?;
    Some(VersionPosition {
        start: version_match.start(),
        end: version_match.end(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_at_position() {
        let content = "version = \"1.2.3\"";
        let pos = VersionPosition { start: 11, end: 16 };
        let result = replace_at_position(content, &pos, "2.0.0");
        assert_eq!(result, "version = \"2.0.0\"");
    }
}
