use std::collections::HashMap;
use std::path::PathBuf;

use super::OutputMode;

/// 命令执行上下文
///
/// 封装命令执行的配置信息，包括程序名、参数、工作目录、环境变量和输出模式。
/// 使用 Builder 模式支持链式调用构建上下文。
///
/// # Example
///
/// ```ignore
/// use domain::runner::{ExecutionContext, OutputMode};
///
/// let ctx = ExecutionContext::new("git")
///     .arg("pull")
///     .working_dir("/path/to/repo")
///     .output_mode(OutputMode::Streaming);
/// ```
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// 要执行的程序名
    pub program: String,

    /// 命令行参数
    pub args: Vec<String>,

    /// 工作目录 (None 表示当前目录)
    pub working_dir: Option<PathBuf>,

    /// 环境变量 (会与当前环境合并)
    pub env_vars: HashMap<String, String>,

    /// 输出模式
    pub output_mode: OutputMode,
}

impl ExecutionContext {
    /// 创建新的执行上下文
    ///
    /// # Arguments
    ///
    /// * `program` - 要执行的程序名
    ///
    /// # Returns
    ///
    /// 返回一个新的 ExecutionContext 实例，使用默认配置：
    /// - 空参数列表
    /// - 无工作目录（使用当前目录）
    /// - 空环境变量
    /// - 默认输出模式 (Capture)
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            working_dir: None,
            env_vars: HashMap::new(),
            output_mode: OutputMode::default(),
        }
    }

    /// 添加单个参数
    ///
    /// # Arguments
    ///
    /// * `arg` - 要添加的参数
    ///
    /// # Returns
    ///
    /// 返回更新后的 ExecutionContext 实例（支持链式调用）
    #[allow(dead_code)]
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// 添加多个参数
    ///
    /// # Arguments
    ///
    /// * `args` - 要添加的参数迭代器
    ///
    /// # Returns
    ///
    /// 返回更新后的 ExecutionContext 实例（支持链式调用）
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// 设置工作目录
    ///
    /// # Arguments
    ///
    /// * `dir` - 工作目录路径
    ///
    /// # Returns
    ///
    /// 返回更新后的 ExecutionContext 实例（支持链式调用）
    pub fn working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// 添加环境变量
    ///
    /// # Arguments
    ///
    /// * `key` - 环境变量名
    /// * `value` - 环境变量值
    ///
    /// # Returns
    ///
    /// 返回更新后的 ExecutionContext 实例（支持链式调用）
    #[allow(dead_code)]
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    /// 设置输出模式
    ///
    /// # Arguments
    ///
    /// * `mode` - 输出模式 (Capture, Streaming, DryRun)
    ///
    /// # Returns
    ///
    /// 返回更新后的 ExecutionContext 实例（支持链式调用）
    pub fn output_mode(mut self, mode: OutputMode) -> Self {
        self.output_mode = mode;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_context_with_program() {
        let ctx = ExecutionContext::new("git");

        assert_eq!(ctx.program, "git");
        assert!(ctx.args.is_empty());
        assert!(ctx.working_dir.is_none());
        assert!(ctx.env_vars.is_empty());
        assert_eq!(ctx.output_mode, OutputMode::Capture);
    }

    #[test]
    fn test_arg_adds_single_argument() {
        let ctx = ExecutionContext::new("git").arg("pull");

        assert_eq!(ctx.args, vec!["pull"]);
    }

    #[test]
    fn test_args_adds_multiple_arguments() {
        let ctx = ExecutionContext::new("git").args(vec!["pull", "--rebase"]);

        assert_eq!(ctx.args, vec!["pull", "--rebase"]);
    }

    #[test]
    fn test_working_dir_sets_directory() {
        let ctx = ExecutionContext::new("git").working_dir("/path/to/repo");

        assert_eq!(ctx.working_dir, Some(PathBuf::from("/path/to/repo")));
    }

    #[test]
    fn test_env_adds_environment_variable() {
        let ctx = ExecutionContext::new("git").env("GIT_AUTHOR_NAME", "Test User");

        assert_eq!(
            ctx.env_vars.get("GIT_AUTHOR_NAME"),
            Some(&"Test User".to_string())
        );
    }

    #[test]
    fn test_output_mode_sets_mode() {
        let ctx = ExecutionContext::new("git").output_mode(OutputMode::Streaming);

        assert_eq!(ctx.output_mode, OutputMode::Streaming);
    }

    #[test]
    fn test_builder_chain() {
        let ctx = ExecutionContext::new("git")
            .arg("pull")
            .arg("--rebase")
            .working_dir("/path/to/repo")
            .env("GIT_AUTHOR_NAME", "Test User")
            .output_mode(OutputMode::Streaming);

        assert_eq!(ctx.program, "git");
        assert_eq!(ctx.args, vec!["pull", "--rebase"]);
        assert_eq!(ctx.working_dir, Some(PathBuf::from("/path/to/repo")));
        assert_eq!(
            ctx.env_vars.get("GIT_AUTHOR_NAME"),
            Some(&"Test User".to_string())
        );
        assert_eq!(ctx.output_mode, OutputMode::Streaming);
    }
}
