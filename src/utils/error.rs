pub struct ErrorHandler;

impl ErrorHandler {
    pub fn print_error(context: &str, error: &dyn std::error::Error) {
        eprintln!("错误: {} - {}", context, error);
    }

    pub fn print_error_anyhow(context: &str, error: &anyhow::Error) {
        eprintln!("错误: {} - {}", context, error);
    }
}
