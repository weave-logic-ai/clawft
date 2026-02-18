//! `weft help [topic]` -- Show help for a topic or general overview.
//!
//! Available topics: `skills`, `agents`, `tools`, `commands`, `config`.
//!
//! When no topic is given, prints a general overview of all subcommands.

use clap::Args;

use crate::help_text;

/// Arguments for `weft help`.
#[derive(Args)]
pub struct HelpArgs {
    /// Topic to display help for (skills, agents, tools, commands, config).
    pub topic: Option<String>,
}

/// Run the help subcommand.
pub fn run(args: HelpArgs) -> anyhow::Result<()> {
    match args.topic {
        Some(topic) => println!("{}", help_text::topic_help(&topic)),
        None => println!("{}", help_text::general_help()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_no_topic_shows_general() {
        // Verify run succeeds with no topic and prints general help.
        let args = HelpArgs { topic: None };
        let result = run(args);
        assert!(result.is_ok());
    }

    #[test]
    fn help_skills_topic() {
        let args = HelpArgs {
            topic: Some("skills".into()),
        };
        let result = run(args);
        assert!(result.is_ok());
    }

    #[test]
    fn help_unknown_topic() {
        let args = HelpArgs {
            topic: Some("nonexistent".into()),
        };
        let result = run(args);
        // Should succeed (prints "no help available" message, does not error).
        assert!(result.is_ok());
    }

    #[test]
    fn help_agents_topic() {
        let args = HelpArgs {
            topic: Some("agents".into()),
        };
        assert!(run(args).is_ok());
    }

    #[test]
    fn help_tools_topic() {
        let args = HelpArgs {
            topic: Some("tools".into()),
        };
        assert!(run(args).is_ok());
    }

    #[test]
    fn help_commands_topic() {
        let args = HelpArgs {
            topic: Some("commands".into()),
        };
        assert!(run(args).is_ok());
    }

    #[test]
    fn help_config_topic() {
        let args = HelpArgs {
            topic: Some("config".into()),
        };
        assert!(run(args).is_ok());
    }
}
