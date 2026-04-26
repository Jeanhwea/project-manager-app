use super::{ConfigEditor, DependencyRef, VersionEditError, VersionLocation, VersionPosition};

pub struct PomXmlEditor;

impl PomXmlEditor {
    fn find_element_position(
        content: &str,
        element: &roxmltree::Node,
    ) -> Option<VersionPosition> {
        let range = element.range();
        let text = element.text()?;
        let full_text = element.document().input_text();

        let element_start: usize = range.start;
        let element_text = &full_text[element_start..];

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
        let before_end = &content[..parent_end];

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
        let doc =
            roxmltree::Document::parse(content).map_err(|e| VersionEditError::ParseError {
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
                if Self::is_inside_parent_element(&node) && parent_version.is_none() {
                    parent_version = Self::find_element_position(content, &node);
                } else if Self::is_inside_dependencies_element(&node) {
                } else if Self::is_direct_child_of_project(&node) && project_version.is_none() {
                    project_version = Self::find_element_position(content, &node);
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
            let mut result = String::new();
            result.push_str(&content[..pos.start]);
            result.push_str(new_version);
            result.push_str(&content[pos.end..]);
            Ok(result)
        } else if location.parent_version.is_some() {
            let parent_end = Self::find_parent_end_tag(content)?;
            let mut result = String::new();
            result.push_str(&content[..parent_end]);

            let indent = Self::detect_indent_before_parent(content, parent_end);
            result.push_str(&format!("\n{}<version>{}</version>", indent, new_version));
            result.push_str(&content[parent_end..]);
            Ok(result)
        } else {
            Err(VersionEditError::VersionNotFound {
                file: "pom.xml".to_string(),
                hint: "pom.xml 未找到项目版本。如果这是继承自父 POM 的项目，请手动添加 <version> 标签。".to_string(),
            })
        }
    }

    fn validate(&self, original: &str, edited: &str) -> Result<(), VersionEditError> {
        if roxmltree::Document::parse(edited).is_err() {
            return Err(VersionEditError::FormatPreservationError {
                file: "pom.xml".to_string(),
            });
        }

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
