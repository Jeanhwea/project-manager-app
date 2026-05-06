#[allow(dead_code)]
pub struct ErrorHandler;

#[allow(dead_code)]
impl ErrorHandler {
    pub fn print_error(context: &str, error: &dyn std::error::Error) {
        eprintln!("错误: {} - {}", context, error);
    }

    pub fn print_error_anyhow(context: &str, error: &anyhow::Error) {
        eprintln!("错误: {} - {}", context, error);
    }

    pub fn print_warning(msg: &str) {
        eprintln!("警告: {}", msg);
    }

    pub fn format_with_chain(error: &dyn std::error::Error) -> String {
        let mut msg = error.to_string();
        let mut source = error.source();
        while let Some(e) = source {
            msg.push_str(&format!("\n  原因: {}", e));
            source = e.source();
        }
        msg
    }

    pub fn format_anyhow_with_chain(error: &anyhow::Error) -> String {
        let mut msg = error.to_string();
        for e in error.chain().skip(1) {
            msg.push_str(&format!("\n  原因: {}", e));
        }
        msg
    }

    pub fn with_context<T, E: std::error::Error>(
        result: Result<T, E>,
        context: &str,
    ) -> anyhow::Result<T> {
        result.map_err(|e| anyhow::anyhow!("{}: {}", context, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_format_with_chain_single_error() {
        let error = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let msg = ErrorHandler::format_with_chain(&error);
        assert!(msg.contains("file not found"));
    }

    #[test]
    fn test_format_with_chain_nested_error() {
        let inner = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");
        let outer: anyhow::Error = anyhow::anyhow!("operation failed").context(inner);

        let msg = ErrorHandler::format_with_chain(outer.as_ref());
        assert!(msg.contains("operation failed"));
    }

    #[test]
    fn test_with_context_ok() {
        let result: Result<i32, io::Error> = Ok(42);
        let wrapped = ErrorHandler::with_context(result, "context");
        assert!(wrapped.is_ok());
        assert_eq!(wrapped.unwrap(), 42);
    }

    #[test]
    fn test_with_context_err() {
        let result: Result<i32, io::Error> =
            Err(io::Error::new(io::ErrorKind::NotFound, "not found"));
        let wrapped = ErrorHandler::with_context(result, "failed to load");
        assert!(wrapped.is_err());
        let err = wrapped.unwrap_err();
        assert!(err.to_string().contains("failed to load"));
        assert!(err.to_string().contains("not found"));
    }
}
