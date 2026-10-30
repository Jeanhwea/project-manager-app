//! 执行计划模型：一次命令被拆解为若干 `Phase`，每个 Phase 由操作（`Step::Op`）和
//! 穿插的展示消息（`Step::Msg`）组成。`Phase` 之间可设置失败是否继续，`ExecutionPlan`
//! 之上还记录了 dry-run 标记、前置/后置消息以及执行后的 `ExecutionResult`。

use crate::model::operation::Operation;

/// 计划内的展示消息，每个变体对应 `utils::output` 中的一种输出样式
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

/// Phase 内的一个步骤：要么是一个待执行的操作，要么是一条只做展示的消息
#[derive(Debug, Clone)]
pub enum Step {
    Op(Operation),
    Msg(DisplayMessage),
}

/// 往编排对象里追加操作/消息的统一抽象，`Phase` 与 `ExecutionPlan` 都实现它
pub trait AddOperation {
    fn add_op(&mut self, op: impl Into<Operation>);
    fn add_msg(&mut self, msg: DisplayMessage);
}

/// 计划中的一个执行阶段，是失败容错的最小单位
#[derive(Debug, Clone)]
pub struct Phase {
    label: String,
    steps: Vec<Step>,
    operation_count: usize,
    continue_on_error: bool,
}

impl Phase {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            steps: Vec::new(),
            operation_count: 0,
            continue_on_error: false,
        }
    }

    pub fn continue_on_error(mut self) -> Self {
        self.continue_on_error = true;
        self
    }

    pub fn is_continue_on_error(&self) -> bool {
        self.continue_on_error
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

/// 整个计划的顶层容器：phases 依次执行，messages 在开头渲染、footer_messages 在结尾渲染
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    phases: Vec<Phase>,
    messages: Vec<DisplayMessage>,
    footer_messages: Vec<DisplayMessage>,
    dry_run: bool,
}

impl ExecutionPlan {
    pub fn new() -> Self {
        Self {
            phases: Vec::new(),
            messages: Vec::new(),
            footer_messages: Vec::new(),
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

    /// 所有 Phase 执行完之后才渲染的消息，用于结果汇总
    pub fn footer_messages(&self) -> &[DisplayMessage] {
        &self.footer_messages
    }

    pub fn add_phase(&mut self, phase: Phase) {
        self.phases.push(phase);
    }

    pub fn add_message(&mut self, msg: DisplayMessage) {
        self.messages.push(msg);
    }

    pub fn add_footer_message(&mut self, msg: DisplayMessage) {
        self.footer_messages.push(msg);
    }

    pub fn add(&mut self, op: impl Into<Operation>) {
        // 未显式创建 Phase 时兜底建一个 default，避免操作无处可放
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

/// 计划执行后的汇总结果：成功数 + 失败明细，供调用方决定如何收尾
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

    pub fn merge(&mut self, other: ExecutionResult) {
        self.executed += other.executed;
        self.errors.extend(other.errors);
    }
}

impl Default for ExecutionResult {
    fn default() -> Self {
        Self::new()
    }
}

/// 单个操作的失败记录，description 是失败的那一步，recovery_hint 提示如何人工补救
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
