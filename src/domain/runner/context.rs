use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputMode {
    #[default]
    Capture,
    Streaming,
}

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub program: String,
    pub args: Vec<String>,
    pub working_dir: Option<PathBuf>,
    pub output_mode: OutputMode,
    pub timeout: Option<Duration>,
}

impl ExecutionContext {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            working_dir: None,
            output_mode: OutputMode::default(),
            timeout: None,
        }
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    pub fn output_mode(mut self, mode: OutputMode) -> Self {
        self.output_mode = mode;
        self
    }

    pub fn timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }
}

#[derive(Debug)]
pub struct CommandResult {
    pub exit_code: i32,
    pub success: bool,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}

impl CommandResult {
    pub fn with_output(exit_code: i32, stdout: String, stderr: String) -> Self {
        Self {
            exit_code,
            success: exit_code == 0,
            stdout: Some(stdout),
            stderr: Some(stderr),
        }
    }
}
