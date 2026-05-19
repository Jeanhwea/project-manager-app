use crate::model::operation::Operation;

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
    pub fn messages(&self) -> &[DisplayMessage] {
        self.metadata.messages()
    }

    pub fn add_phase(&mut self, phase: Phase) {
        self.phases.push(phase);
    }
    pub fn add_message(&mut self, msg: DisplayMessage) {
        self.metadata.add_message(msg);
    }

    pub fn add(&mut self, op: impl Into<Operation>) {
        if self.phases.is_empty() {
            self.phases.push(Phase::new("default"));
        }
        self.phases.last_mut().unwrap().add(op);
    }

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
    executed: usize,
    errors: Vec<OperationError>,
}

impl ExecutionResult {
    pub fn new() -> Self {
        Self {
            executed: 0,
            errors: Vec::new(),
        }
    }
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
    pub fn executed_count(&self) -> usize {
        self.executed
    }
    pub fn errors(&self) -> &[OperationError] {
        &self.errors
    }
    pub fn add_executed(&mut self) {
        self.executed += 1;
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
pub struct OperationError {
    description: String,
    recovery_hint: Option<String>,
}

impl OperationError {
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
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
    pub fn recovery_hint(&self) -> Option<&str> {
        self.recovery_hint.as_deref()
    }
}
