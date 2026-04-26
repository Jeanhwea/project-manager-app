use super::Command;
use anyhow::{anyhow, Result};
use std::collections::HashMap;

pub struct CommandRegistry {
    commands: HashMap<&'static str, Box<dyn Command>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    pub fn register(mut self, command: impl Command + 'static) -> Self {
        let name = command.name();
        self.commands.insert(name, Box::new(command));
        self
    }

    pub fn get(&self, name: &str) -> Option<&Box<dyn Command>> {
        self.commands.get(name)
    }

    pub fn execute(&self, name: &str, ctx: &super::CommandContext) -> Result<()> {
        let command = self
            .commands
            .get(name)
            .ok_or_else(|| anyhow!("未知命令: {}", name))?;
        command.execute(ctx)
    }

    pub fn list(&self) -> Vec<&'static str> {
        self.commands.keys().copied().collect()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}
