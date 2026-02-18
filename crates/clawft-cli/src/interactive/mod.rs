//! Interactive slash-command framework for the `weft agent` REPL.
//!
//! Provides a [`SlashCommandRegistry`] that manages built-in and custom
//! slash commands, plus an [`InteractiveContext`] holding the session state
//! needed by command handlers.

pub mod builtins;
pub mod registry;
