//! 版本号在原文中的位置表示：定位阶段产出字节范围，改写阶段按范围做替换。
//! 由于是纯文本切片，所有偏移都是 UTF-8 字节偏移，替换时必须保持字节边界合法。

#[derive(Debug, Clone, Default)]
/// 一个配置文件里版本号的整体情况。
pub struct VersionLocation {
    /// None 表示该文件没有可改的项目版本号（如 workspace 根）。
    pub project_version: Option<VersionPosition>,
    /// true 表示该文件本身不含版本号（workspace 根），应跳过改写。
    pub is_workspace_root: bool,
}

/// 版本号字符串在原文中的半开字节区间 [start, end)。
#[derive(Debug, Clone)]
pub struct VersionPosition {
    pub start: usize,
    pub end: usize,
}

/// 用 new_value 替换 [start, end) 区间的原文，其余内容原样保留。
pub fn replace_at_position(content: &str, pos: &VersionPosition, new_value: &str) -> String {
    let mut result = String::with_capacity(content.len() + new_value.len());
    result.push_str(&content[..pos.start]);
    result.push_str(new_value);
    result.push_str(&content[pos.end..]);
    result
}

/// 取 regex 第一个捕获组（即版本号本身，不含引号等外围字符）的位置。
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
