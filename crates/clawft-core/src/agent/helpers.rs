//! Template rendering helpers for agent prompts.
//!
//! Provides variable substitution for agent system prompts and skill
//! instructions.  The template syntax is intentionally simple and does
//! not depend on a full template engine:
//!
//! | Syntax        | Replaced with                                     |
//! |---------------|---------------------------------------------------|
//! | `$ARGUMENTS`  | The full argument string passed to the agent       |
//! | `${1}`, `${2}`| Positional arguments (1-based, space-separated)   |
//! | `${NAME}`     | Named variable from `AgentDefinition.variables`   |
//!
//! Missing positional or named variables are replaced with an empty
//! string -- no error is raised.

use std::collections::HashMap;

/// Render template variables in a string.
///
/// Performs three substitution passes in order:
/// 1. `$ARGUMENTS` is replaced with the full `args` string.
/// 2. `${N}` (where N is a positive integer) is replaced with the
///    Nth positional argument (1-based).  Arguments are split on
///    whitespace.
/// 3. `${NAME}` (where NAME is not a number) is replaced with the
///    matching entry from `variables`.
///
/// Unknown variables and out-of-range positional references are
/// replaced with an empty string.
///
/// # Examples
///
/// ```rust,ignore
/// use std::collections::HashMap;
/// let vars = HashMap::from([("lang".to_string(), "Rust".to_string())]);
/// let result = render_template("Hello ${1}, using ${lang}!", "world", &vars);
/// assert_eq!(result, "Hello world, using Rust!");
/// ```
pub fn render_template(template: &str, args: &str, variables: &HashMap<String, String>) -> String {
    // Pass 1: replace $ARGUMENTS
    let result = template.replace("$ARGUMENTS", args);

    // Pre-split positional args for pass 2
    let positional: Vec<&str> = if args.is_empty() {
        Vec::new()
    } else {
        args.split_whitespace().collect()
    };

    // Pass 2 + 3: replace ${...} references
    let mut output = String::with_capacity(result.len());
    let mut chars = result.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' && chars.peek() == Some(&'{') {
            // Consume '{'
            chars.next();

            // Read until '}' or end of string
            let mut var_name = String::new();
            let mut found_close = false;
            for inner in chars.by_ref() {
                if inner == '}' {
                    found_close = true;
                    break;
                }
                var_name.push(inner);
            }

            if !found_close {
                // Malformed -- emit as-is
                output.push('$');
                output.push('{');
                output.push_str(&var_name);
            } else if let Ok(idx) = var_name.parse::<usize>() {
                // Positional argument (1-based)
                if idx >= 1 && idx <= positional.len() {
                    output.push_str(positional[idx - 1]);
                }
                // else: out of range -> empty string
            } else {
                // Named variable
                if let Some(val) = variables.get(&var_name) {
                    output.push_str(val);
                }
                // else: missing -> empty string
            }
        } else {
            output.push(ch);
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_vars() -> HashMap<String, String> {
        HashMap::new()
    }

    fn sample_vars() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("lang".into(), "Rust".into());
        m.insert("framework".into(), "Axum".into());
        m
    }

    #[test]
    fn arguments_substitution() {
        let result = render_template("Run with: $ARGUMENTS", "foo bar baz", &empty_vars());
        assert_eq!(result, "Run with: foo bar baz");
    }

    #[test]
    fn arguments_empty() {
        let result = render_template("Args: $ARGUMENTS end", "", &empty_vars());
        assert_eq!(result, "Args:  end");
    }

    #[test]
    fn positional_arg_1() {
        let result = render_template("Hello ${1}!", "world", &empty_vars());
        assert_eq!(result, "Hello world!");
    }

    #[test]
    fn positional_args_multiple() {
        let result = render_template("${1} and ${2}", "alpha beta gamma", &empty_vars());
        assert_eq!(result, "alpha and beta");
    }

    #[test]
    fn positional_arg_out_of_range() {
        let result = render_template("Missing: ${5}", "only one", &empty_vars());
        assert_eq!(result, "Missing: ");
    }

    #[test]
    fn positional_arg_zero_is_empty() {
        // ${0} is out of range (1-based)
        let result = render_template("Zero: ${0}", "first", &empty_vars());
        assert_eq!(result, "Zero: ");
    }

    #[test]
    fn named_variable_substitution() {
        let result = render_template(
            "Language: ${lang}, Framework: ${framework}",
            "",
            &sample_vars(),
        );
        assert_eq!(result, "Language: Rust, Framework: Axum");
    }

    #[test]
    fn missing_variable_becomes_empty() {
        let result = render_template("Missing: ${nonexistent}", "", &empty_vars());
        assert_eq!(result, "Missing: ");
    }

    #[test]
    fn mixed_substitutions() {
        let result = render_template(
            "Run ${1} with ${lang}: $ARGUMENTS",
            "test_suite extra_arg",
            &sample_vars(),
        );
        assert_eq!(result, "Run test_suite with Rust: test_suite extra_arg");
    }

    #[test]
    fn no_substitutions_passthrough() {
        let template = "No variables here, just plain text.";
        let result = render_template(template, "args", &empty_vars());
        assert_eq!(result, template);
    }

    #[test]
    fn unclosed_brace_preserved() {
        let result = render_template("Broken: ${unclosed", "", &empty_vars());
        assert_eq!(result, "Broken: ${unclosed");
    }

    #[test]
    fn dollar_without_brace_preserved() {
        let result = render_template("Price: $100", "", &empty_vars());
        assert_eq!(result, "Price: $100");
    }

    #[test]
    fn empty_template() {
        let result = render_template("", "args", &sample_vars());
        assert_eq!(result, "");
    }

    #[test]
    fn arguments_appears_multiple_times() {
        let result = render_template("$ARGUMENTS and again $ARGUMENTS", "hello", &empty_vars());
        assert_eq!(result, "hello and again hello");
    }

    #[test]
    fn positional_with_extra_whitespace() {
        // Multiple spaces between args should still split correctly
        let result = render_template("${1} ${2}", "alpha   beta", &empty_vars());
        assert_eq!(result, "alpha beta");
    }

    #[test]
    fn consecutive_variables() {
        let result = render_template("${lang}${framework}", "", &sample_vars());
        assert_eq!(result, "RustAxum");
    }
}
