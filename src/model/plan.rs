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

#[derive(Debug, Clone)]
pub enum Step {
    Op(Operation),
    Msg(DisplayMessage),
}

pub trait AddOperation {
    fn add_op(&mut self, op: impl Into<Operation>);
    fn add_msg(&mut self, msg: DisplayMessage);
}

#[derive(Debug, Clone)]
pub struct Phase {
    label: String,
    steps: Vec<Step>,
    operation_count: usize,
}

impl Phase {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            steps: Vec::new(),
            operation_count: 0,
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    pub fn operation_count(&self) -> usize {
        self.operation_count
    }

    pub fn add(&mut self, op: impl Into<Operation>) {
        self.steps.push(Step::Op(op.into()));
        self.operation_count += 1;
    }

    pub fn add_message(&mut self, msg: DisplayMessage) {
        self.steps.push(Step::Msg(msg));
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
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
pub struct ExecutionPlan {
    phases: Vec<Phase>,
    messages: Vec<DisplayMessage>,
    dry_run: bool,
}

impl ExecutionPlan {
    pub fn new() -> Self {
        Self {
            phases: Vec::new(),
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

    pub fn phases(&self) -> &[Phase] {
        &self.phases
    }

    pub fn messages(&self) -> &[DisplayMessage] {
        &self.messages
    }

    pub fn add_phase(&mut self, phase: Phase) {
        self.phases.push(phase);
    }

    pub fn add_message(&mut self, msg: DisplayMessage) {
        self.messages.push(msg);
    }

    pub fn add(&mut self, op: impl Into<Operation>) {
        if self.phases.is_empty() {
            self.phases.push(Phase::new("default"));
        }
        self.phases.last_mut().unwrap().add(op);
    }

    pub fn operation_count(&self) -> usize {
        self.phases.iter().map(|p| p.operation_count()).sum()
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
