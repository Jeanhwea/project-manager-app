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
