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
        _content: &str,
        _location: &VersionLocation,
        _new_version: &str,
    ) -> Result<String, VersionEditError> {
        todo!("Will be implemented in task 2.3")
    }

    fn validate(&self, _original: &str, _edited: &str) -> Result<(), VersionEditError> {
        todo!("Will be implemented in task 2.3")
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
}
