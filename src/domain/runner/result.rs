/// 命令执行结果
#[derive(Debug)]
pub struct CommandResult {
    pub exit_code: i32,
    pub success: bool,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}

impl CommandResult {
    pub fn success() -> Self {
        Self {
            exit_code: 0,
            success: true,
            stdout: None,
            stderr: None,
        }
    }

    #[allow(dead_code)]
    pub fn failure(exit_code: i32) -> Self {
        Self {
            exit_code,
            success: false,
            stdout: None,
            stderr: None,
        }
    }

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
