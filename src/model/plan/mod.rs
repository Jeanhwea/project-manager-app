pub mod edit;
pub mod git;
pub mod message;
pub mod self_update;
pub mod shell;

pub use edit::EditOperation;
pub use git::GitOperation;
pub use message::MessageOperation;
pub use self_update::SelfUpdateOperation;
pub use shell::ShellOperation;

#[derive(Debug, Clone)]
pub enum Operation {
    Git(GitOperation),
    Shell(ShellOperation),
    Edit(EditOperation),
    SelfUpdate(SelfUpdateOperation),
    Message(MessageOperation),
}

impl Operation {
    pub fn description(&self) -> String {
        match self {
            Operation::Git(op) => op.description(),
            Operation::Shell(op) => op.description(),
            Operation::Edit(op) => op.description(),
            Operation::SelfUpdate(op) => op.description(),
            Operation::Message(op) => op.description(),
        }
    }
}

impl From<GitOperation> for Operation {
    fn from(op: GitOperation) -> Self {
        Operation::Git(op)
    }
}

impl From<ShellOperation> for Operation {
    fn from(op: ShellOperation) -> Self {
        Operation::Shell(op)
    }
}

impl From<EditOperation> for Operation {
    fn from(op: EditOperation) -> Self {
        Operation::Edit(op)
    }
}

impl From<SelfUpdateOperation> for Operation {
    fn from(op: SelfUpdateOperation) -> Self {
        Operation::SelfUpdate(op)
    }
}

impl From<MessageOperation> for Operation {
    fn from(op: MessageOperation) -> Self {
        Operation::Message(op)
    }
}

pub struct ExecutionPlan {
    pub operations: Vec<Operation>,
    pub dry_run: bool,
}

impl ExecutionPlan {
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            dry_run: false,
        }
    }

    pub fn with_dry_run(mut self, value: bool) -> Self {
        self.dry_run = value;
        self
    }

    pub fn add(&mut self, op: impl Into<Operation>) {
        self.operations.push(op.into());
    }
}

impl Default for ExecutionPlan {
    fn default() -> Self {
        Self::new()
    }
}
