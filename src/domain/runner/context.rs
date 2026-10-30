//! 外部命令调用的入参与出参描述：builder 风格组装上下文，避免到处散传参数。

use std::path::PathBuf;
use std::time::Duration;

/// 子进程输出的处理方式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputMode {
    /// 收集到内存里由调用方解析
    #[default]
    Capture,
    /// 边产出边转发到终端，事后拿不到内容
    Streaming,
}

/// 一次外部命令调用的完整描述
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub program: String,
    pub args: Vec<String>,
    pub working_dir: Option<PathBuf>,
    pub output_mode: OutputMode,
    /// 超时上限，None 表示不限时；只对 Streaming 模式生效
    pub timeout: Option<Duration>,
}

impl ExecutionContext {
    /// 以程序名为起点，其余参数通过 builder 逐个追加
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            working_dir: None,
            output_mode: OutputMode::default(),
            timeout: None,
        }
    }

    /// 追加命令行参数，可重复调用
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// 指定子进程工作目录；不设置则继承当前进程的工作目录
    pub fn working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// 切换输出处理方式
    pub fn output_mode(mut self, mode: OutputMode) -> Self {
        self.output_mode = mode;
        self
    }

    /// 设置超时上限，None 表示不限时
    pub fn timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }
}

/// 命令执行结果
#[derive(Debug)]
pub struct CommandResult {
    pub exit_code: i32,
    pub success: bool,
    /// 仅 Capture 模式有值，Streaming 模式的输出已实时转发走
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}

impl CommandResult {
    /// 按捕获到的输出构造结果；退出码为 0 视为成功
    pub fn with_output(exit_code: i32, stdout: String, stderr: String) -> Self {
        Self {
            exit_code,
            success: exit_code == 0,
            stdout: Some(stdout),
            stderr: Some(stderr),
        }
    }
}
