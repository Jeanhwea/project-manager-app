/// 命令执行结果
///
/// 封装命令执行的结果信息，包括退出码、成功状态和可选的输出内容。
///
/// # Output Mode Behavior
///
/// - **Capture Mode**: `stdout` and `stderr` contain the captured output
/// - **Streaming Mode**: `stdout` and `stderr` are `None` (output was displayed in real-time)
/// - **DryRun Mode**: `stdout` and `stderr` are `None`, `exit_code` is always 0
///
/// # Example
///
/// ```ignore
/// use domain::runner::CommandResult;
///
/// // Create a success result
/// let result = CommandResult::success();
/// assert!(result.success);
///
/// // Create a failure result
/// let result = CommandResult::failure(1);
/// assert!(!result.success);
/// assert_eq!(result.exit_code, 1);
///
/// // Create a result with captured output
/// let result = CommandResult::with_output(0, "stdout content".to_string(), "".to_string());
/// assert!(result.success);
/// assert_eq!(result.stdout, Some("stdout content".to_string()));
/// ```
#[derive(Debug)]
pub struct CommandResult {
    /// 退出码
    ///
    /// 0 表示成功，非零表示失败。
    /// 对于 DryRun 模式，始终为 0。
    pub exit_code: i32,

    /// 是否成功 (exit_code == 0)
    pub success: bool,

    /// 标准输出 (仅在 Capture 模式下有值)
    ///
    /// 在 Streaming 和 DryRun 模式下为 `None`。
    pub stdout: Option<String>,

    /// 标准错误 (仅在 Capture 模式下有值)
    ///
    /// 在 Streaming 和 DryRun 模式下为 `None`。
    pub stderr: Option<String>,
}

impl CommandResult {
    /// 创建成功的结果
    ///
    /// 用于 DryRun 模式或不需要捕获输出的成功场景。
    ///
    /// # Returns
    ///
    /// 返回一个 `CommandResult` 实例：
    /// - `exit_code`: 0
    /// - `success`: true
    /// - `stdout`: None
    /// - `stderr`: None
    pub fn success() -> Self {
        Self {
            exit_code: 0,
            success: true,
            stdout: None,
            stderr: None,
        }
    }

    /// 创建失败的结果
    ///
    /// 用于命令执行失败且不需要捕获输出的场景。
    ///
    /// # Arguments
    ///
    /// * `exit_code` - 非零退出码
    ///
    /// # Returns
    ///
    /// 返回一个 `CommandResult` 实例：
    /// - `exit_code`: 传入的退出码
    /// - `success`: false
    /// - `stdout`: None
    /// - `stderr`: None
    #[allow(dead_code)]
    pub fn failure(exit_code: i32) -> Self {
        Self {
            exit_code,
            success: false,
            stdout: None,
            stderr: None,
        }
    }

    /// 创建带捕获输出的结果
    ///
    /// 用于 Capture 模式下命令执行完成后的结果。
    ///
    /// # Arguments
    ///
    /// * `exit_code` - 退出码（0 表示成功，非零表示失败）
    /// * `stdout` - 标准输出内容
    /// * `stderr` - 标准错误内容
    ///
    /// # Returns
    ///
    /// 返回一个 `CommandResult` 实例：
    /// - `exit_code`: 传入的退出码
    /// - `success`: true (如果 exit_code == 0) 或 false (如果 exit_code != 0)
    /// - `stdout`: Some(传入的 stdout)
    /// - `stderr`: Some(传入的 stderr)
    pub fn with_output(exit_code: i32, stdout: String, stderr: String) -> Self {
        Self {
            exit_code,
            success: exit_code == 0,
            stdout: Some(stdout),
            stderr: Some(stderr),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_creates_successful_result() {
        let result = CommandResult::success();

        assert_eq!(result.exit_code, 0);
        assert!(result.success);
        assert!(result.stdout.is_none());
        assert!(result.stderr.is_none());
    }

    #[test]
    fn test_failure_creates_failed_result() {
        let result = CommandResult::failure(1);

        assert_eq!(result.exit_code, 1);
        assert!(!result.success);
        assert!(result.stdout.is_none());
        assert!(result.stderr.is_none());
    }

    #[test]
    fn test_failure_with_different_exit_codes() {
        let result = CommandResult::failure(127);
        assert_eq!(result.exit_code, 127);
        assert!(!result.success);

        let result = CommandResult::failure(-1);
        assert_eq!(result.exit_code, -1);
        assert!(!result.success);
    }

    #[test]
    fn test_with_output_creates_result_with_output() {
        let result = CommandResult::with_output(
            0,
            "stdout content".to_string(),
            "stderr content".to_string(),
        );

        assert_eq!(result.exit_code, 0);
        assert!(result.success);
        assert_eq!(result.stdout, Some("stdout content".to_string()));
        assert_eq!(result.stderr, Some("stderr content".to_string()));
    }

    #[test]
    fn test_with_output_non_zero_exit_code() {
        let result =
            CommandResult::with_output(1, "some output".to_string(), "error message".to_string());

        assert_eq!(result.exit_code, 1);
        assert!(!result.success);
        assert_eq!(result.stdout, Some("some output".to_string()));
        assert_eq!(result.stderr, Some("error message".to_string()));
    }

    #[test]
    fn test_with_output_empty_strings() {
        let result = CommandResult::with_output(0, "".to_string(), "".to_string());

        assert!(result.success);
        assert_eq!(result.stdout, Some("".to_string()));
        assert_eq!(result.stderr, Some("".to_string()));
    }
}
