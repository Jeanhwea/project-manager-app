// Re-export operation types for backward compatibility
pub use crate::model::operation::{
    EditOperation, GitOperation, Operation, SelfUpdateOperation, ShellOperation,
};

#[derive(Debug, Clone)]
pub enum DisplayMessage {
    Header {
        title: String,
    },
    Section {
        title: String,
    },
    Item {
        label: String,
        value: String,
    },
    Detail {
        label: String,
        value: String,
    },
    Diff {
        file: String,
        old_start: usize,
        new_start: usize,
        old_lines: Vec<String>,
        new_lines: Vec<String>,
        old_count: usize,
        new_count: usize,
    },
    Success {
        msg: String,
    },
    Warning {
        msg: String,
    },
    Skip {
        msg: String,
    },
    Blank,
}

impl DisplayMessage {
    pub fn description(&self) -> String {
        match self {
            DisplayMessage::Header { title } => title.clone(),
            DisplayMessage::Section { title } => title.clone(),
            DisplayMessage::Item { label, value } => format!("{}: {}", label, value),
            DisplayMessage::Detail { label, value } => format!("  {}: {}", label, value),
            DisplayMessage::Diff {
                file,
                old_start,
                new_start,
                old_lines,
                new_lines,
                old_count,
                new_count,
            } => {
                let mut diff_lines = vec![
                    format!("diff --git a/{} b/{}", file, file),
                    format!("--- a/{}", file),
                    format!("+++ b/{}", file),
                    format!(
                        "@@ -{},{} +{},{} @@",
                        old_start, old_count, new_start, new_count
                    ),
                ];

                for line in old_lines {
                    diff_lines.push(format!("-{}", line));
                }

                for line in new_lines {
                    diff_lines.push(format!("+{}", line));
                }

                diff_lines.join("\n")
            }
            DisplayMessage::Success { msg } => format!("OK> {}", msg),
            DisplayMessage::Warning { msg } => format!("WARN {}", msg),
            DisplayMessage::Skip { msg } => format!("SKIP {}", msg),
            DisplayMessage::Blank => String::new(),
        }
    }
}

pub trait AddOperation {
    fn add_op(&mut self, op: impl Into<Operation>);
    fn add_msg(&mut self, msg: DisplayMessage);
}

#[derive(Debug, Clone)]
pub struct Phase {
    label: String,
    operations: Vec<Operation>,
    messages: Vec<DisplayMessage>,
}

impl Phase {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            operations: Vec::new(),
            messages: Vec::new(),
        }
    }
    pub fn label(&self) -> &str {
        &self.label
    }
    pub fn operations(&self) -> &[Operation] {
        &self.operations
    }
    pub fn messages(&self) -> &[DisplayMessage] {
        &self.messages
    }
    pub fn add(&mut self, op: impl Into<Operation>) {
        self.operations.push(op.into());
    }
    pub fn add_message(&mut self, msg: DisplayMessage) {
        self.messages.push(msg);
    }
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty() && self.messages.is_empty()
    }
}

impl AddOperation for Phase {
    fn add_op(&mut self, op: impl Into<Operation>) {
        self.add(op);
    }
    fn add_msg(&mut self, msg: DisplayMessage) {
        self.add_message(msg);
    }
}

#[derive(Debug, Clone)]
pub struct PlanMetadata {
    messages: Vec<DisplayMessage>,
    dry_run: bool,
}

impl PlanMetadata {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            dry_run: false,
        }
    }
    pub fn with_dry_run(mut self, value: bool) -> Self {
        self.dry_run = value;
        self
    }
    pub fn dry_run(&self) -> bool {
        self.dry_run
    }
    pub fn messages(&self) -> &[DisplayMessage] {
        &self.messages
    }
    pub fn add_message(&mut self, msg: DisplayMessage) {
        self.messages.push(msg);
    }
}

impl Default for PlanMetadata {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    phases: Vec<Phase>,
    metadata: PlanMetadata,
}

impl ExecutionPlan {
    pub fn new() -> Self {
        Self {
            phases: Vec::new(),
            metadata: PlanMetadata::new(),
        }
    }
    pub fn with_dry_run(mut self, value: bool) -> Self {
        self.metadata = self.metadata.with_dry_run(value);
        self
    }
    pub fn dry_run(&self) -> bool {
        self.metadata.dry_run()
    }
    pub fn phases(&self) -> &[Phase] {
        &self.phases
    }
    pub fn metadata(&self) -> &PlanMetadata {
        &self.metadata
    }
    pub fn messages(&self) -> &[DisplayMessage] {
        self.metadata.messages()
    }

    pub fn add_phase(&mut self, phase: Phase) {
        self.phases.push(phase);
    }
    pub fn add_message(&mut self, msg: DisplayMessage) {
        self.metadata.add_message(msg);
    }

    /// Add an operation to the last phase, creating a default phase if none exists
    pub fn add(&mut self, op: impl Into<Operation>) {
        if self.phases.is_empty() {
            self.phases.push(Phase::new("default"));
        }
        self.phases.last_mut().unwrap().add(op);
    }

    pub fn all_operations(&self) -> Vec<&Operation> {
        self.phases.iter().flat_map(|p| p.operations()).collect()
    }

    /// Count non-message operations (for dry-run header)
    pub fn operation_count(&self) -> usize {
        self.phases.iter().map(|p| p.operations.len()).sum()
    }
}

impl Default for ExecutionPlan {
    fn default() -> Self {
        Self::new()
    }
}

impl AddOperation for ExecutionPlan {
    fn add_op(&mut self, op: impl Into<Operation>) {
        self.add(op);
    }
    fn add_msg(&mut self, msg: DisplayMessage) {
        self.add_message(msg);
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    executed: Vec<ExecutedOperation>,
    skipped: Vec<SkippedOperation>,
    errors: Vec<OperationError>,
}

impl ExecutionResult {
    pub fn new() -> Self {
        Self {
            executed: Vec::new(),
            skipped: Vec::new(),
            errors: Vec::new(),
        }
    }
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
    pub fn executed(&self) -> &[ExecutedOperation] {
        &self.executed
    }
    pub fn skipped(&self) -> &[SkippedOperation] {
        &self.skipped
    }
    pub fn errors(&self) -> &[OperationError] {
        &self.errors
    }
    pub fn add_executed(&mut self, op: ExecutedOperation) {
        self.executed.push(op);
    }
    pub fn add_skipped(&mut self, op: SkippedOperation) {
        self.skipped.push(op);
    }
    pub fn add_error(&mut self, err: OperationError) {
        self.errors.push(err);
    }
}

impl Default for ExecutionResult {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ExecutedOperation {
    description: String,
    phase: String,
}

impl ExecutedOperation {
    pub fn new(description: impl Into<String>, phase: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            phase: phase.into(),
        }
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn phase(&self) -> &str {
        &self.phase
    }
}

#[derive(Debug, Clone)]
pub struct SkippedOperation {
    description: String,
    reason: String,
}

impl SkippedOperation {
    pub fn new(description: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            reason: reason.into(),
        }
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

#[derive(Debug, Clone)]
pub struct OperationError {
    description: String,
    phase: String,
    recovery_hint: Option<String>,
}

impl OperationError {
    pub fn new(description: impl Into<String>, phase: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            phase: phase.into(),
            recovery_hint: None,
        }
    }
    pub fn with_recovery_hint(mut self, hint: impl Into<String>) -> Self {
        self.recovery_hint = Some(hint.into());
        self
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn phase(&self) -> &str {
        &self.phase
    }
    pub fn recovery_hint(&self) -> Option<&str> {
        self.recovery_hint.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_message_diff_description_includes_file_and_line_numbers() {
        let diff = DisplayMessage::Diff {
            file: "pyproject.toml".to_string(),
            old_start: 3,
            new_start: 3,
            old_lines: vec!["version = \"1.4.10\"".to_string()],
            new_lines: vec!["version = \"1.5.0\"".to_string()],
            old_count: 1,
            new_count: 1,
        };

        let expected = "diff --git a/pyproject.toml b/pyproject.toml\n--- a/pyproject.toml\n+++ b/pyproject.toml\n@@ -3,1 +3,1 @@\n-version = \"1.4.10\"\n+version = \"1.5.0\"";
        assert_eq!(diff.description(), expected);
    }
}
