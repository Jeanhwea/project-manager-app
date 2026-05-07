use std::collections::HashMap;
use std::path::PathBuf;

use super::OutputMode;

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub program: String,
    pub args: Vec<String>,
    pub working_dir: Option<PathBuf>,
    pub env_vars: HashMap<String, String>,
    pub output_mode: OutputMode,
}

impl ExecutionContext {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            working_dir: None,
            env_vars: HashMap::new(),
            output_mode: OutputMode::default(),
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
    fn test_output_mode_sets_mode() {
        let ctx = ExecutionContext::new("git").output_mode(OutputMode::Streaming);

        assert_eq!(ctx.output_mode, OutputMode::Streaming);
    }

    #[test]
    fn test_builder_chain() {
        let ctx = ExecutionContext::new("git")
            .args(vec!["pull", "--rebase"])
            .working_dir("/path/to/repo")
            .output_mode(OutputMode::Streaming);

        assert_eq!(ctx.program, "git");
        assert_eq!(ctx.args, vec!["pull", "--rebase"]);
        assert_eq!(ctx.working_dir, Some(PathBuf::from("/path/to/repo")));
        assert_eq!(ctx.output_mode, OutputMode::Streaming);
    }
}
