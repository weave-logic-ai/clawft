//! Slash command registry and dispatch.
//!
//! The [`SlashCommandRegistry`] holds a set of named [`SlashCommand`]
//! implementations and dispatches user input that starts with `/` to the
//! matching handler.
//!
//! # Example
//!
//! ```rust,ignore
//! let mut registry = SlashCommandRegistry::new();
//! registry.register(Box::new(HelpCommand));
//! let result = registry.dispatch("/help", &mut ctx);
//! ```

use std::collections::HashMap;

/// Mutable context passed to slash commands during execution.
///
/// Holds references to the session state that commands may inspect or
/// modify (e.g. the active agent, active skill, available tool names).
pub struct InteractiveContext {
    /// Currently active agent name (empty = default).
    pub active_agent: String,

    /// Currently active skill name (empty = none).
    pub active_skill: String,

    /// Current model identifier.
    pub model: String,

    /// Registered tool names (for `/tools` display).
    pub tool_names: Vec<String>,

    /// Registered skill names (for `/skills` display).
    pub skill_names: Vec<String>,

    /// Registered agent names (for `/agent` display).
    pub agent_names: Vec<String>,
}

impl InteractiveContext {
    /// Create a new context with the given model.
    pub fn new(model: String) -> Self {
        Self {
            active_agent: String::new(),
            active_skill: String::new(),
            model,
            tool_names: Vec::new(),
            skill_names: Vec::new(),
            agent_names: Vec::new(),
        }
    }
}

/// Trait for a slash command handler.
///
/// Implementors provide a name (without the `/` prefix), a description
/// for help text, and an [`execute`](SlashCommand::execute) method that
/// processes the command arguments and returns output to display.
pub trait SlashCommand: Send + Sync {
    /// Command name without the leading `/`.
    fn name(&self) -> &str;

    /// One-line description for help text.
    fn description(&self) -> &str;

    /// Execute the command with the given arguments string.
    ///
    /// Returns the text to display to the user, or an error.
    fn execute(&self, args: &str, ctx: &mut InteractiveContext) -> anyhow::Result<String>;
}

/// Registry of slash commands with dispatch.
pub struct SlashCommandRegistry {
    commands: HashMap<String, Box<dyn SlashCommand>>,
}

impl SlashCommandRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    /// Register a slash command.
    ///
    /// If a command with the same name already exists, it is replaced.
    pub fn register(&mut self, cmd: Box<dyn SlashCommand>) {
        self.commands.insert(cmd.name().to_string(), cmd);
    }

    /// Register a slash command, checking for collision with existing commands.
    ///
    /// Returns `Err(collision_name)` if a command with the same name is already
    /// registered (the new command is NOT registered in this case).
    /// Returns `Ok(())` if the command was registered successfully.
    pub fn register_checked(&mut self, cmd: Box<dyn SlashCommand>) -> Result<(), String> {
        let name = cmd.name().to_string();
        if self.commands.contains_key(&name) {
            return Err(name);
        }
        self.commands.insert(name, cmd);
        Ok(())
    }

    /// Dispatch a line of input.
    ///
    /// The input should start with `/`. The first word (after `/`) is the
    /// command name, and the rest is passed as `args`.
    ///
    /// Returns `Some(output)` if the command was found and executed, or
    /// `None` if the command is not registered.
    pub fn dispatch(
        &self,
        input: &str,
        ctx: &mut InteractiveContext,
    ) -> Option<anyhow::Result<String>> {
        let input = input.trim();
        if !input.starts_with('/') {
            return None;
        }

        let without_slash = &input[1..];
        let (name, args) = match without_slash.split_once(char::is_whitespace) {
            Some((n, a)) => (n, a.trim()),
            None => (without_slash, ""),
        };

        let cmd = self.commands.get(name)?;
        Some(cmd.execute(args, ctx))
    }

    /// Check whether a command is registered.
    pub fn has(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }

    /// List all registered command names (sorted).
    pub fn names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.commands.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    /// Get a reference to a command by name.
    pub fn get(&self, name: &str) -> Option<&dyn SlashCommand> {
        self.commands.get(name).map(|b| b.as_ref())
    }

    /// Number of registered commands.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

impl Default for SlashCommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A test command that echoes its arguments.
    struct EchoCommand;

    impl SlashCommand for EchoCommand {
        fn name(&self) -> &str {
            "echo"
        }
        fn description(&self) -> &str {
            "Echo arguments back"
        }
        fn execute(&self, args: &str, _ctx: &mut InteractiveContext) -> anyhow::Result<String> {
            Ok(format!("echo: {args}"))
        }
    }

    /// A test command that returns an error.
    struct FailCommand;

    impl SlashCommand for FailCommand {
        fn name(&self) -> &str {
            "fail"
        }
        fn description(&self) -> &str {
            "Always fails"
        }
        fn execute(&self, _args: &str, _ctx: &mut InteractiveContext) -> anyhow::Result<String> {
            anyhow::bail!("intentional failure")
        }
    }

    fn test_ctx() -> InteractiveContext {
        InteractiveContext::new("test-model".into())
    }

    #[test]
    fn registry_starts_empty() {
        let reg = SlashCommandRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        assert!(reg.names().is_empty());
    }

    #[test]
    fn register_and_lookup() {
        let mut reg = SlashCommandRegistry::new();
        reg.register(Box::new(EchoCommand));

        assert!(!reg.is_empty());
        assert_eq!(reg.len(), 1);
        assert!(reg.has("echo"));
        assert!(!reg.has("nonexistent"));
        assert_eq!(reg.names(), vec!["echo"]);
    }

    #[test]
    fn dispatch_known_command() {
        let mut reg = SlashCommandRegistry::new();
        reg.register(Box::new(EchoCommand));
        let mut ctx = test_ctx();

        let result = reg.dispatch("/echo hello world", &mut ctx);
        assert!(result.is_some());
        assert_eq!(result.unwrap().unwrap(), "echo: hello world");
    }

    #[test]
    fn dispatch_command_no_args() {
        let mut reg = SlashCommandRegistry::new();
        reg.register(Box::new(EchoCommand));
        let mut ctx = test_ctx();

        let result = reg.dispatch("/echo", &mut ctx);
        assert!(result.is_some());
        assert_eq!(result.unwrap().unwrap(), "echo: ");
    }

    #[test]
    fn dispatch_unknown_command_returns_none() {
        let reg = SlashCommandRegistry::new();
        let mut ctx = test_ctx();

        let result = reg.dispatch("/unknown", &mut ctx);
        assert!(result.is_none());
    }

    #[test]
    fn dispatch_non_slash_input_returns_none() {
        let mut reg = SlashCommandRegistry::new();
        reg.register(Box::new(EchoCommand));
        let mut ctx = test_ctx();

        let result = reg.dispatch("regular message", &mut ctx);
        assert!(result.is_none());
    }

    #[test]
    fn dispatch_error_propagates() {
        let mut reg = SlashCommandRegistry::new();
        reg.register(Box::new(FailCommand));
        let mut ctx = test_ctx();

        let result = reg.dispatch("/fail", &mut ctx);
        assert!(result.is_some());
        assert!(result.unwrap().is_err());
    }

    #[test]
    fn register_replaces_existing() {
        let mut reg = SlashCommandRegistry::new();
        reg.register(Box::new(EchoCommand));
        reg.register(Box::new(EchoCommand));
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn names_are_sorted() {
        let mut reg = SlashCommandRegistry::new();
        reg.register(Box::new(EchoCommand));
        reg.register(Box::new(FailCommand));

        let names = reg.names();
        assert_eq!(names, vec!["echo", "fail"]);
    }

    #[test]
    fn get_returns_command_ref() {
        let mut reg = SlashCommandRegistry::new();
        reg.register(Box::new(EchoCommand));

        let cmd = reg.get("echo");
        assert!(cmd.is_some());
        assert_eq!(cmd.unwrap().name(), "echo");
        assert_eq!(cmd.unwrap().description(), "Echo arguments back");
    }

    #[test]
    fn get_missing_returns_none() {
        let reg = SlashCommandRegistry::new();
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn dispatch_trims_input() {
        let mut reg = SlashCommandRegistry::new();
        reg.register(Box::new(EchoCommand));
        let mut ctx = test_ctx();

        let result = reg.dispatch("  /echo trimmed  ", &mut ctx);
        assert!(result.is_some());
        assert_eq!(result.unwrap().unwrap(), "echo: trimmed");
    }

    #[test]
    fn default_creates_empty_registry() {
        let reg = SlashCommandRegistry::default();
        assert!(reg.is_empty());
    }

    #[test]
    fn interactive_context_new() {
        let ctx = InteractiveContext::new("model/v1".into());
        assert_eq!(ctx.model, "model/v1");
        assert!(ctx.active_agent.is_empty());
        assert!(ctx.active_skill.is_empty());
        assert!(ctx.tool_names.is_empty());
        assert!(ctx.skill_names.is_empty());
        assert!(ctx.agent_names.is_empty());
    }

    #[test]
    fn interactive_context_mutability() {
        let mut ctx = InteractiveContext::new("model/v1".into());
        ctx.active_agent = "researcher".into();
        ctx.active_skill = "research".into();
        ctx.tool_names = vec!["read_file".into()];

        assert_eq!(ctx.active_agent, "researcher");
        assert_eq!(ctx.active_skill, "research");
        assert_eq!(ctx.tool_names, vec!["read_file"]);
    }
}
