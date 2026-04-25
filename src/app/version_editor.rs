pub trait ConfigEditor {
    fn parse(&self, content: &str) -> Result<VersionLocation, VersionEditError>;
    fn edit(
        &self,
        content: &str,
        location: &VersionLocation,
        new_version: &str,
    ) -> Result<String, VersionEditError>;
    fn validate(&self, original: &str, edited: &str) -> Result<(), VersionEditError>;
}

pub struct VersionLocation {
    pub project_version: Option<VersionPosition>,
    pub parent_version: Option<VersionPosition>,
    pub is_workspace_root: bool,
    pub dependency_refs: Vec<DependencyRef>,
}

pub struct VersionPosition {
    pub start: usize,
    pub end: usize,
    pub line: usize,
}

pub struct DependencyRef {
    pub name_pattern: String,
    pub position: VersionPosition,
}

#[derive(Debug)]
pub enum VersionEditError {
    FileNotFound(String),
    ParseError { file: String, reason: String },
    VersionNotFound { file: String, hint: String },
    WriteError { file: String, reason: String },
    FormatPreservationError { file: String },
}

pub struct PomXmlEditor;

impl PomXmlEditor {
    fn find_element_position(content: &str, element: &roxmltree::Node) -> Option<VersionPosition> {
        let range = element.range();
        let text = element.text()?;
        let full_text = element.document().input_text();
        
        // Find the text content position within the element
        let element_start: usize = range.start.into();
        let element_text = &full_text[element_start..];
        
        // Find where the text content starts (after >)
        if let Some(text_start_offset) = element_text.find('>') {
            let text_start = element_start + text_start_offset + 1;
            let text_end = text_start + text.len();
            let line = content[..text_start].chars().filter(|&c| c == '\n').count() + 1;
            return Some(VersionPosition {
                start: text_start,
                end: text_end,
                line,
            });
        }
        None
    }

    fn is_direct_child_of_project(node: &roxmltree::Node) -> bool {
        let Some(parent) = node.parent() else {
            return false;
        };
        parent.tag_name().name() == "project"
    }

    fn is_inside_parent_element(node: &roxmltree::Node) -> bool {
        let mut current = node.parent();
        while let Some(parent) = current {
            if parent.tag_name().name() == "parent" {
                return true;
            }
            current = parent.parent();
        }
        false
    }

    fn is_inside_dependencies_element(node: &roxmltree::Node) -> bool {
        let mut current = node.parent();
        while let Some(parent) = current {
            if parent.tag_name().name() == "dependencies" {
                return true;
            }
            current = parent.parent();
        }
        false
    }

    fn find_parent_end_tag(content: &str) -> Result<usize, VersionEditError> {
        // Find the position right after </parent>
        let end_tag = "</parent>";
        if let Some(pos) = content.find(end_tag) {
            Ok(pos + end_tag.len())
        } else {
            Err(VersionEditError::ParseError {
                file: "pom.xml".to_string(),
                reason: "Could not find </parent> tag".to_string(),
            })
        }
    }

    fn detect_indent_before_parent(content: &str, parent_end: usize) -> String {
        // Look backwards from </parent> to find the indentation
        let before_end = &content[..parent_end];
        
        // Find the start of the line containing </parent>
        let line_start = before_end.rfind('\n').map(|p| p + 1).unwrap_or(0);
        let indent: String = before_end[line_start..]
            .chars()
            .take_while(|c| c.is_whitespace() && *c != '\n' && *c != '\r')
            .collect();
        
        indent
    }
}

impl ConfigEditor for PomXmlEditor {
    fn parse(&self, content: &str) -> Result<VersionLocation, VersionEditError> {
        let doc = roxmltree::Document::parse(content).map_err(|e| VersionEditError::ParseError {
            file: "pom.xml".to_string(),
            reason: e.to_string(),
        })?;

        let root = doc.root_element();
        if root.tag_name().name() != "project" {
            return Err(VersionEditError::ParseError {
                file: "pom.xml".to_string(),
                reason: "Root element is not <project>".to_string(),
            });
        }

        let mut project_version: Option<VersionPosition> = None;
        let mut parent_version: Option<VersionPosition> = None;

        for node in root.descendants() {
            if node.tag_name().name() == "version" {
                if Self::is_inside_parent_element(&node) {
                    if parent_version.is_none() {
                        parent_version = Self::find_element_position(content, &node);
                    }
                } else if Self::is_inside_dependencies_element(&node) {
                    // Skip dependency versions
                } else if Self::is_direct_child_of_project(&node) {
                    if project_version.is_none() {
                        project_version = Self::find_element_position(content, &node);
                    }
                }
            }
        }

        Ok(VersionLocation {
            project_version,
            parent_version,
            is_workspace_root: false,
            dependency_refs: Vec::new(),
        })
    }

    fn edit(
        &self,
        content: &str,
        location: &VersionLocation,
        new_version: &str,
    ) -> Result<String, VersionEditError> {
        if let Some(ref pos) = location.project_version {
            // Case 1: Project version exists - replace it
            let mut result = String::new();
            result.push_str(&content[..pos.start]);
            result.push_str(new_version);
            result.push_str(&content[pos.end..]);
            Ok(result)
        } else if location.parent_version.is_some() {
            // Case 2: No project version but has parent version - insert after </parent>
            let parent_end = Self::find_parent_end_tag(content)?;
            let mut result = String::new();
            result.push_str(&content[..parent_end]);
            
            // Detect indentation style from parent element
            let indent = Self::detect_indent_before_parent(content, parent_end);
            result.push_str(&format!("\n{}<version>{}</version>", indent, new_version));
            result.push_str(&content[parent_end..]);
            Ok(result)
        } else {
            // No version found at all
            Err(VersionEditError::VersionNotFound {
                file: "pom.xml".to_string(),
                hint: "pom.xml 未找到项目版本。如果这是继承自父 POM 的项目，请手动添加 <version> 标签。".to_string(),
            })
        }
    }

    fn validate(&self, original: &str, edited: &str) -> Result<(), VersionEditError> {
        // Check that the edited content is valid XML
        if roxmltree::Document::parse(edited).is_err() {
            return Err(VersionEditError::FormatPreservationError {
                file: "pom.xml".to_string(),
            });
        }

        // Check newline style preservation
        let original_has_crlf = original.contains("\r\n");
        let edited_has_crlf = edited.contains("\r\n");
        if original_has_crlf != edited_has_crlf && original_has_crlf {
            return Err(VersionEditError::FormatPreservationError {
                file: "pom.xml".to_string(),
            });
        }

        Ok(())
    }
}

pub struct CargoTomlEditor;

impl CargoTomlEditor {
    fn find_version_position(content: &str, doc: &toml_edit::DocumentMut) -> Option<VersionPosition> {
        let package = doc.get("package")?.as_table_like()?;
        
        // Check if version key exists
        if !package.contains_key("version") {
            return None;
        }
        
        // Find the version in the raw content
        // Look for 'version = "value"' pattern in [package] section
        let version_pattern = regex::Regex::new(r#"version\s*=\s*"[^"]*""#).ok()?;
        
        // Find the [package] section start
        let package_start = content.find("[package]")?;
        let package_end = content[package_start..]
            .find("\n[")
            .map(|p| package_start + p)
            .unwrap_or(content.len());
        
        let package_section = &content[package_start..package_end];
        
        if let Some(m) = version_pattern.find(package_section) {
            let start = package_start + m.start();
            let end = package_start + m.end();
            let line = content[..start].chars().filter(|&c| c == '\n').count() + 1;
            return Some(VersionPosition { start, end, line });
        }
        
        None
    }
}

impl ConfigEditor for CargoTomlEditor {
    fn parse(&self, content: &str) -> Result<VersionLocation, VersionEditError> {
        let doc = content.parse::<toml_edit::DocumentMut>().map_err(|e| VersionEditError::ParseError {
            file: "Cargo.toml".to_string(),
            reason: e.to_string(),
        })?;

        // Check if [package] section exists
        let has_package = doc.contains_key("package");
        let has_workspace = doc.contains_key("workspace");

        if !has_package && has_workspace {
            // This is a workspace root file
            return Ok(VersionLocation {
                project_version: None,
                parent_version: None,
                is_workspace_root: true,
                dependency_refs: Vec::new(),
            });
        }

        if !has_package {
            return Err(VersionEditError::VersionNotFound {
                file: "Cargo.toml".to_string(),
                hint: "Cargo.toml 未找到 [package] section。".to_string(),
            });
        }

        let project_version = Self::find_version_position(content, &doc);

        if project_version.is_none() {
            return Err(VersionEditError::VersionNotFound {
                file: "Cargo.toml".to_string(),
                hint: "Cargo.toml [package] section 中未找到 version 字段。".to_string(),
            });
        }

        Ok(VersionLocation {
            project_version,
            parent_version: None,
            is_workspace_root: false,
            dependency_refs: Vec::new(),
        })
    }

    fn edit(
        &self,
        content: &str,
        location: &VersionLocation,
        new_version: &str,
    ) -> Result<String, VersionEditError> {
        if location.is_workspace_root {
            return Err(VersionEditError::VersionNotFound {
                file: "Cargo.toml".to_string(),
                hint: "Cargo.toml 是 workspace 根文件，无项目版本。请指定具体的 member package。".to_string(),
            });
        }

        let mut doc = content.parse::<toml_edit::DocumentMut>().map_err(|e| VersionEditError::ParseError {
            file: "Cargo.toml".to_string(),
            reason: e.to_string(),
        })?;

        if let Some(package) = doc.get_mut("package") {
            if let Some(table) = package.as_table_like_mut() {
                table.insert("version", toml_edit::value(new_version));
            }
        }

        Ok(doc.to_string())
    }

    fn validate(&self, original: &str, edited: &str) -> Result<(), VersionEditError> {
        // Check that the edited content is valid TOML
        if edited.parse::<toml_edit::DocumentMut>().is_err() {
            return Err(VersionEditError::FormatPreservationError {
                file: "Cargo.toml".to_string(),
            });
        }

        // Check newline style preservation
        let original_has_crlf = original.contains("\r\n");
        let edited_has_crlf = edited.contains("\r\n");
        if original_has_crlf != edited_has_crlf && original_has_crlf {
            return Err(VersionEditError::FormatPreservationError {
                file: "Cargo.toml".to_string(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pom_with_project_and_parent_version() {
        let content = r#"
<project>
    <parent>
        <version>1.0.0</version>
    </parent>
    <version>2.0.0</version>
    <dependencies>
        <dependency>
            <version>3.0.0</version>
        </dependency>
    </dependencies>
</project>
"#;
        let editor = PomXmlEditor;
        let location = editor.parse(content).unwrap();

        assert!(location.project_version.is_some());
        assert!(location.parent_version.is_some());
    }

    #[test]
    fn test_parse_pom_with_only_parent_version() {
        let content = r#"
<project>
    <parent>
        <version>1.0.0</version>
    </parent>
</project>
"#;
        let editor = PomXmlEditor;
        let location = editor.parse(content).unwrap();

        assert!(location.project_version.is_none());
        assert!(location.parent_version.is_some());
    }

    #[test]
    fn test_parse_pom_ignores_dependency_versions() {
        let content = r#"
<project>
    <version>2.0.0</version>
    <dependencies>
        <dependency>
            <version>3.0.0</version>
        </dependency>
    </dependencies>
</project>
"#;
        let editor = PomXmlEditor;
        let location = editor.parse(content).unwrap();

        assert!(location.project_version.is_some());
        assert!(location.parent_version.is_none());
    }

    #[test]
    fn test_parse_pom_with_nested_dependencies() {
        let content = r#"
<project>
    <version>1.0.0</version>
    <dependencyManagement>
        <dependencies>
            <dependency>
                <version>2.0.0</version>
            </dependency>
        </dependencies>
    </dependencyManagement>
    <build>
        <plugins>
            <plugin>
                <version>3.0.0</version>
            </plugin>
        </plugins>
    </build>
</project>
"#;
        let editor = PomXmlEditor;
        let location = editor.parse(content).unwrap();

        assert!(location.project_version.is_some());
        // Verify the project version is "1.0.0"
        let pos = location.project_version.unwrap();
        let version_text = &content[pos.start..pos.end];
        assert_eq!(version_text, "1.0.0");
    }

    #[test]
    fn test_parse_pom_no_version() {
        let content = r#"
<project>
    <groupId>com.example</groupId>
    <artifactId>test</artifactId>
</project>
"#;
        let editor = PomXmlEditor;
        let location = editor.parse(content).unwrap();

        assert!(location.project_version.is_none());
        assert!(location.parent_version.is_none());
    }

    #[test]
    fn test_parse_pom_invalid_xml() {
        let content = r#"
<project>
    <version>1.0.0
</project>
"#;
        let editor = PomXmlEditor;
        let result = editor.parse(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_pom_non_project_root() {
        let content = r#"
<notproject>
    <version>1.0.0</version>
</notproject>
"#;
        let editor = PomXmlEditor;
        let result = editor.parse(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_edit_pom_with_project_version() {
        let content = r#"
<project>
    <parent>
        <version>1.0.0</version>
    </parent>
    <version>2.0.0</version>
    <dependencies>
        <dependency>
            <version>3.0.0</version>
        </dependency>
    </dependencies>
</project>
"#;
        let editor = PomXmlEditor;
        let location = editor.parse(content).unwrap();
        let edited = editor.edit(content, &location, "2.1.0").unwrap();

        assert!(edited.contains("<version>2.1.0</version>"));
        assert!(edited.contains("<version>1.0.0</version>")); // parent unchanged
        assert!(edited.contains("<version>3.0.0</version>")); // dependency unchanged
    }

    #[test]
    fn test_edit_pom_insert_after_parent() {
        let content = r#"
<project>
    <parent>
        <version>1.0.0</version>
    </parent>
</project>
"#;
        let editor = PomXmlEditor;
        let location = editor.parse(content).unwrap();
        assert!(location.project_version.is_none());
        assert!(location.parent_version.is_some());

        let edited = editor.edit(content, &location, "2.0.0").unwrap();
        assert!(edited.contains("</parent>\n    <version>2.0.0</version>"));
        assert!(edited.contains("<version>1.0.0</version>")); // parent unchanged
    }

    #[test]
    fn test_edit_pom_no_version_error() {
        let content = r#"
<project>
    <groupId>com.example</groupId>
    <artifactId>test</artifactId>
</project>
"#;
        let editor = PomXmlEditor;
        let location = editor.parse(content).unwrap();
        let result = editor.edit(content, &location, "2.0.0");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_valid_xml() {
        let content = r#"
<project>
    <version>1.0.0</version>
</project>
"#;
        let editor = PomXmlEditor;
        let location = editor.parse(content).unwrap();
        let edited = editor.edit(content, &location, "2.0.0").unwrap();
        assert!(editor.validate(content, &edited).is_ok());
    }

    #[test]
    fn test_validate_preserves_crlf() {
        let content = "<project>\r\n    <version>1.0.0</version>\r\n</project>\r\n";
        let editor = PomXmlEditor;
        let location = editor.parse(content).unwrap();
        let edited = editor.edit(content, &location, "2.0.0").unwrap();
        assert!(edited.contains("\r\n"));
        assert!(editor.validate(content, &edited).is_ok());
    }

    // CargoTomlEditor tests

    #[test]
    fn test_parse_cargo_with_package() {
        let content = r#"
[package]
name = "test"
version = "1.0.0"

[dependencies]
serde = "1.0"
"#;
        let editor = CargoTomlEditor;
        let location = editor.parse(content).unwrap();

        assert!(location.project_version.is_some());
        assert!(!location.is_workspace_root);
    }

    #[test]
    fn test_parse_cargo_workspace_root() {
        let content = r#"
[workspace]
members = ["crate1", "crate2"]
"#;
        let editor = CargoTomlEditor;
        let location = editor.parse(content).unwrap();

        assert!(location.project_version.is_none());
        assert!(location.is_workspace_root);
    }

    #[test]
    fn test_parse_cargo_no_package_no_workspace() {
        let content = r#"
[dependencies]
serde = "1.0"
"#;
        let editor = CargoTomlEditor;
        let result = editor.parse(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_cargo_package_no_version() {
        let content = r#"
[package]
name = "test"
"#;
        let editor = CargoTomlEditor;
        let result = editor.parse(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_edit_cargo_with_version() {
        let content = r#"
[package]
name = "test"
version = "1.0.0"

[dependencies]
serde = "1.0"
"#;
        let editor = CargoTomlEditor;
        let location = editor.parse(content).unwrap();
        let edited = editor.edit(content, &location, "2.0.0").unwrap();

        assert!(edited.contains("version = \"2.0.0\""));
        assert!(edited.contains("serde = \"1.0\"")); // dependency unchanged
    }

    #[test]
    fn test_edit_cargo_workspace_root_error() {
        let content = r#"
[workspace]
members = ["crate1"]
"#;
        let editor = CargoTomlEditor;
        let location = editor.parse(content).unwrap();
        let result = editor.edit(content, &location, "2.0.0");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_cargo_valid_toml() {
        let content = r#"
[package]
name = "test"
version = "1.0.0"
"#;
        let editor = CargoTomlEditor;
        let location = editor.parse(content).unwrap();
        let edited = editor.edit(content, &location, "2.0.0").unwrap();
        assert!(editor.validate(content, &edited).is_ok());
    }
}
