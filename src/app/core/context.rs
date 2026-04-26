use std::collections::HashMap;
use std::path::PathBuf;

pub struct CommandContext {
    pub working_dir: PathBuf,
    pub args: HashMap<String, String>,
    pub flags: HashMap<String, bool>,
    pub multi_args: HashMap<String, Vec<String>>,
}

impl CommandContext {
    pub fn new() -> Self {
        Self {
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            args: HashMap::new(),
            flags: HashMap::new(),
            multi_args: HashMap::new(),
        }
    }

    pub fn arg(mut self, key: &str, value: &str) -> Self {
        self.args.insert(key.to_string(), value.to_string());
        self
    }

    pub fn flag(mut self, key: &str, value: bool) -> Self {
        self.flags.insert(key.to_string(), value);
        self
    }

    pub fn multi_arg(mut self, key: &str, values: Vec<String>) -> Self {
        self.multi_args.insert(key.to_string(), values);
        self
    }

    pub fn working_dir(mut self, dir: PathBuf) -> Self {
        self.working_dir = dir;
        self
    }

    pub fn get_arg(&self, key: &str) -> Option<&String> {
        self.args.get(key)
    }

    pub fn get_flag(&self, key: &str) -> bool {
        self.flags.get(key).copied().unwrap_or(false)
    }

    pub fn get_multi_arg(&self, key: &str) -> Option<&Vec<String>> {
        self.multi_args.get(key)
    }
}

impl Default for CommandContext {
    fn default() -> Self {
        Self::new()
    }
}
