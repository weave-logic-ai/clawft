//! Command execution security policy.
//!
//! Provides configurable allowlist/denylist-based command validation with
//! defense-in-depth dangerous pattern detection. Used by shell and spawn
//! tools to gate which executables may be invoked.

use std::collections::HashSet;

use thiserror::Error;

/// Whether the policy operates in allowlist or denylist mode.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum PolicyMode {
    /// Only commands whose basename appears in the allowlist are permitted.
    #[default]
    Allowlist,
    /// All commands are permitted unless they match a denylist pattern.
    Denylist,
}

/// Errors returned when a command fails policy validation.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum PolicyError {
    /// The command's executable is not on the allowlist.
    #[error("command not allowed: {command}")]
    NotAllowed { command: String },

    /// The command matched a denylist pattern.
    #[error("command blocked: {command} (matched pattern: {pattern})")]
    Blocked { command: String, pattern: String },

    /// The command matched a dangerous pattern (defense-in-depth check).
    #[error("dangerous command: {command} (matched pattern: {pattern})")]
    DangerousPattern { command: String, pattern: String },
}

/// Configurable command execution policy.
///
/// Validates commands against an allowlist or denylist, and always checks
/// a set of dangerous patterns regardless of mode (defense-in-depth).
#[derive(Debug, Clone)]
pub struct CommandPolicy {
    /// Operating mode for the policy.
    pub mode: PolicyMode,
    /// Set of permitted executable basenames (used in `Allowlist` mode).
    pub allowlist: HashSet<String>,
    /// Patterns to block (substring match, case-insensitive; used in `Denylist` mode).
    pub denylist: Vec<String>,
    /// Patterns that are always checked regardless of mode (defense-in-depth).
    pub dangerous_patterns: Vec<String>,
}

/// The default set of safe executable basenames for allowlist mode.
const DEFAULT_ALLOWLIST: &[&str] = &[
    "echo", "cat", "ls", "pwd", "head", "tail", "wc", "grep", "find", "sort", "uniq", "diff",
    "date", "env", "true", "false", "test",
];

/// The default set of dangerous patterns, mirroring those in `shell_tool.rs`.
const DEFAULT_DANGEROUS_PATTERNS: &[&str] = &[
    "rm -rf /",
    "sudo ",
    "mkfs",
    "dd if=",
    ":(){ :|:& };:",
    "chmod 777 /",
    "> /dev/sd",
    "shutdown",
    "reboot",
    "poweroff",
    "format c:",
];

impl CommandPolicy {
    /// Create a policy with safe defaults.
    ///
    /// - Mode: `Allowlist`
    /// - Allowlist: common read-only / informational commands
    /// - Dangerous patterns: the standard 11 patterns from `shell_tool.rs`
    /// - Denylist: same patterns (used when mode is switched to `Denylist`)
    pub fn safe_defaults() -> Self {
        let allowlist = DEFAULT_ALLOWLIST.iter().map(|s| (*s).to_string()).collect();
        let dangerous_patterns: Vec<String> = DEFAULT_DANGEROUS_PATTERNS
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let denylist = dangerous_patterns.clone();

        Self {
            mode: PolicyMode::Allowlist,
            allowlist,
            denylist,
            dangerous_patterns,
        }
    }

    /// Create a new policy with explicit configuration.
    pub fn new(mode: PolicyMode, allowlist: HashSet<String>, denylist: Vec<String>) -> Self {
        let dangerous_patterns: Vec<String> = DEFAULT_DANGEROUS_PATTERNS
            .iter()
            .map(|s| (*s).to_string())
            .collect();

        Self {
            mode,
            allowlist,
            denylist,
            dangerous_patterns,
        }
    }

    /// Validate a command string against this policy.
    ///
    /// 1. Always checks dangerous patterns first (defense-in-depth).
    /// 2. In `Allowlist` mode, extracts the executable basename and checks the allowlist.
    /// 3. In `Denylist` mode, checks all denylist patterns (case-insensitive substring match).
    pub fn validate(&self, command: &str) -> Result<(), PolicyError> {
        // Normalize whitespace (tabs, etc.) to spaces for pattern matching,
        // so that "sudo\tsomething" matches the "sudo " pattern.
        let normalized: String = command
            .chars()
            .map(|c| if c.is_whitespace() { ' ' } else { c })
            .collect();
        let lower = normalized.to_lowercase();

        // Step 1: Always check dangerous patterns (defense-in-depth).
        for pattern in &self.dangerous_patterns {
            if lower.contains(&pattern.to_lowercase()) {
                return Err(PolicyError::DangerousPattern {
                    command: command.to_string(),
                    pattern: pattern.clone(),
                });
            }
        }

        // Step 2: Mode-specific checks.
        match self.mode {
            PolicyMode::Allowlist => {
                let token = extract_first_token(command);
                if !self.allowlist.contains(token) {
                    return Err(PolicyError::NotAllowed {
                        command: command.to_string(),
                    });
                }
            }
            PolicyMode::Denylist => {
                for pattern in &self.denylist {
                    if lower.contains(&pattern.to_lowercase()) {
                        return Err(PolicyError::Blocked {
                            command: command.to_string(),
                            pattern: pattern.clone(),
                        });
                    }
                }
            }
        }

        Ok(())
    }
}

/// Extract the first whitespace-delimited token from a command string,
/// stripping any leading path components (basename extraction).
///
/// # Examples
///
/// ```text
/// "echo foo"       -> "echo"
/// "/usr/bin/ls -la" -> "ls"
/// "  cat file"     -> "cat"
/// ""               -> ""
/// ```
pub(crate) fn extract_first_token(command: &str) -> &str {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return "";
    }

    let token = trimmed.split_whitespace().next().unwrap_or("");

    // Strip path prefix: take everything after the last '/'.
    match token.rfind('/') {
        Some(pos) => &token[pos + 1..],
        None => token,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- safe_defaults ---------------------------------------------------------

    #[test]
    fn safe_defaults_creates_correct_mode() {
        let policy = CommandPolicy::safe_defaults();
        assert_eq!(policy.mode, PolicyMode::Allowlist);
    }

    #[test]
    fn safe_defaults_has_expected_allowlist() {
        let policy = CommandPolicy::safe_defaults();
        for cmd in DEFAULT_ALLOWLIST {
            assert!(
                policy.allowlist.contains(*cmd),
                "{cmd} should be in allowlist"
            );
        }
    }

    #[test]
    fn safe_defaults_has_dangerous_patterns() {
        let policy = CommandPolicy::safe_defaults();
        assert_eq!(policy.dangerous_patterns.len(), 11);
    }

    #[test]
    fn safe_defaults_denylist_matches_dangerous() {
        let policy = CommandPolicy::safe_defaults();
        assert_eq!(policy.denylist, policy.dangerous_patterns);
    }

    // -- allowlist mode --------------------------------------------------------

    #[test]
    fn allowlist_permits_echo() {
        let policy = CommandPolicy::safe_defaults();
        assert!(policy.validate("echo hello").is_ok());
    }

    #[test]
    fn allowlist_permits_ls() {
        let policy = CommandPolicy::safe_defaults();
        assert!(policy.validate("ls -la").is_ok());
    }

    #[test]
    fn allowlist_permits_cat() {
        let policy = CommandPolicy::safe_defaults();
        assert!(policy.validate("cat file.txt").is_ok());
    }

    #[test]
    fn allowlist_permits_pwd() {
        let policy = CommandPolicy::safe_defaults();
        assert!(policy.validate("pwd").is_ok());
    }

    #[test]
    fn allowlist_rejects_curl() {
        let policy = CommandPolicy::safe_defaults();
        let err = policy.validate("curl http://evil.com").unwrap_err();
        assert!(matches!(err, PolicyError::NotAllowed { .. }));
    }

    #[test]
    fn allowlist_rejects_wget() {
        let policy = CommandPolicy::safe_defaults();
        let err = policy.validate("wget http://evil.com").unwrap_err();
        assert!(matches!(err, PolicyError::NotAllowed { .. }));
    }

    #[test]
    fn allowlist_rejects_nc() {
        let policy = CommandPolicy::safe_defaults();
        let err = policy.validate("nc -l 4444").unwrap_err();
        assert!(matches!(err, PolicyError::NotAllowed { .. }));
    }

    #[test]
    fn allowlist_rejects_python3() {
        let policy = CommandPolicy::safe_defaults();
        let err = policy.validate("python3 -c \"evil\"").unwrap_err();
        assert!(matches!(err, PolicyError::NotAllowed { .. }));
    }

    #[test]
    fn allowlist_rejects_bash() {
        let policy = CommandPolicy::safe_defaults();
        let err = policy.validate("bash -c \"evil\"").unwrap_err();
        assert!(matches!(err, PolicyError::NotAllowed { .. }));
    }

    // -- denylist mode ---------------------------------------------------------

    #[test]
    fn denylist_permits_curl_when_not_denied() {
        let mut policy = CommandPolicy::safe_defaults();
        policy.mode = PolicyMode::Denylist;
        assert!(policy.validate("curl http://safe.com").is_ok());
    }

    #[test]
    fn denylist_blocks_rm_rf_root() {
        let mut policy = CommandPolicy::safe_defaults();
        policy.mode = PolicyMode::Denylist;
        let err = policy.validate("rm -rf /").unwrap_err();
        // Dangerous patterns are checked first, so this will be DangerousPattern.
        assert!(matches!(err, PolicyError::DangerousPattern { .. }));
    }

    #[test]
    fn denylist_blocks_sudo() {
        let mut policy = CommandPolicy::safe_defaults();
        policy.mode = PolicyMode::Denylist;
        let err = policy.validate("sudo something").unwrap_err();
        assert!(matches!(err, PolicyError::DangerousPattern { .. }));
    }

    // -- extract_first_token ---------------------------------------------------

    #[test]
    fn extract_first_token_simple() {
        assert_eq!(extract_first_token("echo foo"), "echo");
    }

    #[test]
    fn extract_first_token_with_path() {
        assert_eq!(extract_first_token("/usr/bin/ls -la"), "ls");
    }

    #[test]
    fn extract_first_token_leading_whitespace() {
        assert_eq!(extract_first_token("  cat file"), "cat");
    }

    #[test]
    fn extract_first_token_empty() {
        assert_eq!(extract_first_token(""), "");
    }

    #[test]
    fn extract_first_token_whitespace_only() {
        assert_eq!(extract_first_token("   "), "");
    }

    // -- dangerous patterns always checked -------------------------------------

    #[test]
    fn dangerous_patterns_checked_in_allowlist_mode() {
        let policy = CommandPolicy::safe_defaults();
        // "echo" is on the allowlist but the command contains a dangerous pattern.
        let err = policy.validate("echo; rm -rf /").unwrap_err();
        assert!(matches!(err, PolicyError::DangerousPattern { .. }));
    }

    #[test]
    fn dangerous_patterns_checked_in_denylist_mode() {
        let mut policy = CommandPolicy::safe_defaults();
        policy.mode = PolicyMode::Denylist;
        let err = policy.validate("dd if=/dev/zero of=/dev/sda").unwrap_err();
        assert!(matches!(err, PolicyError::DangerousPattern { .. }));
    }

    // -- case insensitivity ----------------------------------------------------

    #[test]
    fn case_insensitive_sudo_blocked() {
        let policy = CommandPolicy::safe_defaults();
        let err = policy.validate("SUDO something").unwrap_err();
        assert!(matches!(err, PolicyError::DangerousPattern { .. }));
    }

    #[test]
    fn case_insensitive_mixed_case_blocked() {
        let policy = CommandPolicy::safe_defaults();
        let err = policy.validate("SuDo apt install evil").unwrap_err();
        assert!(matches!(err, PolicyError::DangerousPattern { .. }));
    }

    // -- path traversal / basename extraction in allowlist ----------------------

    #[test]
    fn allowlist_rejects_path_to_unlisted_binary() {
        let policy = CommandPolicy::safe_defaults();
        // /usr/bin/curl -> basename "curl", which is not on the allowlist.
        let err = policy
            .validate("/usr/bin/curl http://evil.com")
            .unwrap_err();
        assert!(matches!(err, PolicyError::NotAllowed { .. }));
    }

    #[test]
    fn allowlist_permits_path_to_listed_binary() {
        let policy = CommandPolicy::safe_defaults();
        // /usr/bin/ls -> basename "ls", which IS on the allowlist.
        assert!(policy.validate("/usr/bin/ls -la").is_ok());
    }

    // -- tab-separated commands ------------------------------------------------

    #[test]
    fn tab_in_sudo_command_still_matched() {
        let policy = CommandPolicy::safe_defaults();
        // Whitespace is normalized before pattern matching, so "sudo\t"
        // becomes "sudo " which matches the "sudo " dangerous pattern.
        let result = policy.validate("sudo\tsomething");
        assert!(result.is_err(), "tab-separated sudo should be blocked");
    }
}
