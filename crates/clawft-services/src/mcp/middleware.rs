//! Middleware pipeline for MCP tool calls.
//!
//! Provides a composable chain of middlewares that can filter visible tools,
//! inspect/modify/reject tool calls before execution, and post-process
//! results after execution.
//!
//! Built-in middlewares:
//! - [`SecurityGuard`]: enforces command and URL safety policies
//! - [`PermissionFilter`]: restricts visible tools to an allowlist
//! - [`ResultGuard`]: truncates oversized tool output
//! - [`AuditLog`]: logs tool invocations at `info!` level

use std::collections::HashSet;

use async_trait::async_trait;
use tracing::info;

use super::ToolDefinition;
use super::provider::{CallToolResult, ContentBlock, ToolError};

// ---------------------------------------------------------------------------
// Middleware trait
// ---------------------------------------------------------------------------

/// A request to call a tool, passed through the middleware chain.
#[derive(Debug, Clone)]
pub struct ToolCallRequest {
    /// Tool name.
    pub name: String,
    /// JSON arguments for the tool.
    pub args: serde_json::Value,
}

/// Composable middleware for MCP tool operations.
///
/// Each method has a default pass-through implementation so that
/// concrete middlewares only need to override the hooks they care about.
#[async_trait]
pub trait Middleware: Send + Sync {
    /// Filter the list of visible tools.
    async fn filter_tools(&self, tools: Vec<ToolDefinition>) -> Vec<ToolDefinition> {
        tools
    }

    /// Inspect/modify/reject a tool call before execution.
    async fn before_call(&self, request: ToolCallRequest) -> Result<ToolCallRequest, ToolError> {
        Ok(request)
    }

    /// Inspect/modify the result after execution.
    async fn after_call(
        &self,
        _request: &ToolCallRequest,
        result: CallToolResult,
    ) -> Result<CallToolResult, ToolError> {
        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// Simplified security policy types (local to this crate)
// ---------------------------------------------------------------------------

/// Simplified command execution policy for middleware use.
///
/// Since `clawft-services` does not depend on `clawft-tools`, this is a
/// local mirror of the essential validation logic from
/// `clawft_tools::security_policy::CommandPolicy`.
#[derive(Debug, Clone)]
pub struct CommandPolicy {
    /// Allowed command basenames. If empty, all commands are rejected.
    pub allowed_commands: HashSet<String>,
    /// Dangerous patterns that are always rejected (case-insensitive substring).
    pub dangerous_patterns: Vec<String>,
}

impl Default for CommandPolicy {
    fn default() -> Self {
        let allowed_commands: HashSet<String> = [
            "echo", "cat", "ls", "pwd", "head", "tail", "wc", "grep", "find", "sort", "uniq",
            "diff", "date", "env", "true", "false", "test",
        ]
        .iter()
        .map(|s| (*s).to_string())
        .collect();

        let dangerous_patterns = vec![
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
        ]
        .into_iter()
        .map(String::from)
        .collect();

        Self {
            allowed_commands,
            dangerous_patterns,
        }
    }
}

impl CommandPolicy {
    /// Validate a command string against this policy.
    ///
    /// Returns `Ok(())` if the command passes, or a descriptive error string.
    pub fn validate(&self, command: &str) -> Result<(), String> {
        let normalized: String = command
            .chars()
            .map(|c| if c.is_whitespace() { ' ' } else { c })
            .collect();
        let lower = normalized.to_lowercase();

        // Check dangerous patterns first.
        for pattern in &self.dangerous_patterns {
            if lower.contains(&pattern.to_lowercase()) {
                return Err(format!("dangerous pattern: {pattern}"));
            }
        }

        // Extract basename of the first token.
        let basename = command
            .split_whitespace()
            .next()
            .unwrap_or("")
            .rsplit('/')
            .next()
            .unwrap_or("");

        if !self.allowed_commands.contains(basename) {
            return Err(format!("command not allowed: {basename}"));
        }

        Ok(())
    }
}

/// Simplified URL safety policy for middleware use.
///
/// Since `clawft-services` does not depend on `clawft-tools`, this is a
/// local mirror of the essential validation logic from
/// `clawft_tools::url_safety::UrlPolicy`.
#[derive(Debug, Clone)]
pub struct UrlPolicy {
    /// Whether URL safety checks are active.
    pub enabled: bool,
    /// Domains that are always blocked.
    pub blocked_domains: HashSet<String>,
}

impl Default for UrlPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            blocked_domains: HashSet::new(),
        }
    }
}

impl UrlPolicy {
    /// Validate a URL string against this policy.
    ///
    /// Performs basic checks: blocks private IPs and metadata endpoints.
    /// For full SSRF protection, prefer the `clawft_tools::url_safety`
    /// module at the tool-execution layer.
    pub fn validate(&self, url_str: &str) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        // Extract host from the URL. We do a lightweight parse here to
        // avoid pulling in the `url` crate as a dependency.
        let host = extract_host(url_str).unwrap_or_default();

        if self.blocked_domains.contains(&host) {
            return Err(format!("blocked domain: {host}"));
        }

        // Block cloud metadata endpoints.
        const METADATA_HOSTS: &[&str] = &[
            "169.254.169.254",
            "metadata.google.internal",
            "metadata.internal",
        ];
        if METADATA_HOSTS.iter().any(|&m| m == host) {
            return Err(format!("blocked metadata endpoint: {host}"));
        }

        // Block private, loopback, and link-local IP addresses.
        if is_private_or_loopback(&host) {
            return Err(format!("blocked private address: {host}"));
        }

        Ok(())
    }
}

/// Check if a host string represents a private, loopback, or link-local address.
///
/// Handles:
/// - IPv4: 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16, 127.0.0.0/8
/// - IPv6 loopback: ::1
/// - IPv4-mapped IPv6: ::ffff:x.x.x.x (converts to IPv4 and re-checks)
/// - Link-local: 169.254.0.0/16
/// - Special: 0.0.0.0, localhost
fn is_private_or_loopback(host: &str) -> bool {
    // Direct matches
    if host == "0.0.0.0" || host == "localhost" || host == "::1" {
        return true;
    }

    // IPv4-mapped IPv6: ::ffff:x.x.x.x
    if let Some(ipv4_part) = host.strip_prefix("::ffff:") {
        return is_private_or_loopback(ipv4_part);
    }

    // Parse IPv4 octets
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() == 4
        && let (Ok(o1), Ok(o2)) = (parts[0].parse::<u8>(), parts[1].parse::<u8>())
    {
        // 10.0.0.0/8
        if o1 == 10 {
            return true;
        }
        // 172.16.0.0/12 (172.16.* through 172.31.*)
        if o1 == 172 && (16..=31).contains(&o2) {
            return true;
        }
        // 192.168.0.0/16
        if o1 == 192 && o2 == 168 {
            return true;
        }
        // 127.0.0.0/8
        if o1 == 127 {
            return true;
        }
        // 169.254.0.0/16 (link-local)
        if o1 == 169 && o2 == 254 {
            return true;
        }
    }

    false
}

/// Lightweight host extraction from a URL string.
fn extract_host(url: &str) -> Option<String> {
    // Strip scheme.
    let after_scheme = url.find("://").map(|i| &url[i + 3..]).unwrap_or(url);

    // Take up to the next '/', '?', '#', or end.
    let authority = after_scheme
        .split(&['/', '?', '#'][..])
        .next()
        .unwrap_or(after_scheme);

    // Strip optional userinfo (user:pass@).
    let host_port = authority.rsplit('@').next().unwrap_or(authority);

    // Strip port.
    // Be careful with IPv6 bracket notation: [::1]:8080
    let host = if host_port.starts_with('[') {
        // IPv6 bracket notation.
        host_port
            .find(']')
            .map(|i| &host_port[1..i])
            .unwrap_or(host_port)
    } else {
        host_port.rsplit(':').next_back().unwrap_or(host_port)
    };

    if host.is_empty() {
        None
    } else {
        Some(host.to_lowercase())
    }
}

// ---------------------------------------------------------------------------
// SecurityGuard
// ---------------------------------------------------------------------------

/// Middleware that enforces command and URL safety policies.
///
/// - For tools whose name contains "exec" or "shell", validates the
///   `command` argument against [`CommandPolicy`].
/// - For tools whose name contains "fetch" or "search", validates the
///   `url` argument against [`UrlPolicy`].
#[derive(Debug, Clone)]
pub struct SecurityGuard {
    command_policy: CommandPolicy,
    url_policy: UrlPolicy,
}

impl SecurityGuard {
    /// Create a new security guard with the given policies.
    pub fn new(command_policy: CommandPolicy, url_policy: UrlPolicy) -> Self {
        Self {
            command_policy,
            url_policy,
        }
    }
}

impl Default for SecurityGuard {
    fn default() -> Self {
        Self::new(CommandPolicy::default(), UrlPolicy::default())
    }
}

#[async_trait]
impl Middleware for SecurityGuard {
    async fn before_call(&self, request: ToolCallRequest) -> Result<ToolCallRequest, ToolError> {
        let name_lower = request.name.to_lowercase();

        // Check command-execution tools.
        if (name_lower.contains("exec") || name_lower.contains("shell"))
            && let Some(cmd) = request.args.get("command").and_then(|v| v.as_str())
        {
            self.command_policy.validate(cmd).map_err(|reason| {
                ToolError::PermissionDenied {
                    tool: request.name.clone(),
                    reason: format!("command rejected: {reason}"),
                }
            })?;
        }

        // Check URL-fetching tools.
        if (name_lower.contains("fetch") || name_lower.contains("search"))
            && let Some(url) = request.args.get("url").and_then(|v| v.as_str())
        {
            self.url_policy.validate(url).map_err(|reason| {
                ToolError::PermissionDenied {
                    tool: request.name.clone(),
                    reason: format!("URL rejected: {reason}"),
                }
            })?;
        }

        Ok(request)
    }
}

// ---------------------------------------------------------------------------
// PermissionFilter
// ---------------------------------------------------------------------------

/// Middleware that restricts visible tools to an allowlist.
///
/// When `allowed_tools` is `Some`, only tools whose names appear in the
/// list are returned from `filter_tools`. When `None`, all tools pass.
#[derive(Debug, Clone)]
pub struct PermissionFilter {
    allowed_tools: Option<HashSet<String>>,
}

impl PermissionFilter {
    /// Create a new permission filter.
    ///
    /// Pass `None` to allow all tools, or `Some(list)` to restrict.
    pub fn new(allowed_tools: Option<Vec<String>>) -> Self {
        Self {
            allowed_tools: allowed_tools.map(|v| v.into_iter().collect()),
        }
    }
}

#[async_trait]
impl Middleware for PermissionFilter {
    async fn filter_tools(&self, tools: Vec<ToolDefinition>) -> Vec<ToolDefinition> {
        match &self.allowed_tools {
            None => tools,
            Some(allowed) => tools
                .into_iter()
                .filter(|t| allowed.contains(&t.name))
                .collect(),
        }
    }
}

// ---------------------------------------------------------------------------
// ResultGuard
// ---------------------------------------------------------------------------

/// Middleware that truncates oversized tool output.
///
/// If any text content block exceeds `max_bytes`, it is truncated and a
/// `\n[truncated]` sentinel is appended.
#[derive(Debug, Clone)]
pub struct ResultGuard {
    max_bytes: usize,
}

impl ResultGuard {
    /// Create a new result guard with the given byte limit.
    pub fn new(max_bytes: usize) -> Self {
        Self { max_bytes }
    }
}

impl Default for ResultGuard {
    fn default() -> Self {
        Self {
            max_bytes: 64 * 1024, // 64 KB
        }
    }
}

#[async_trait]
impl Middleware for ResultGuard {
    async fn after_call(
        &self,
        _request: &ToolCallRequest,
        result: CallToolResult,
    ) -> Result<CallToolResult, ToolError> {
        let content = result
            .content
            .into_iter()
            .map(|block| match block {
                ContentBlock::Text { text } if text.len() > self.max_bytes => {
                    // Truncate to max_bytes on a char boundary.
                    let mut end = self.max_bytes;
                    while end > 0 && !text.is_char_boundary(end) {
                        end -= 1;
                    }
                    let truncated = format!("{}\n[truncated]", &text[..end]);
                    ContentBlock::Text { text: truncated }
                }
                other => other,
            })
            .collect();

        Ok(CallToolResult {
            content,
            is_error: result.is_error,
        })
    }
}

// ---------------------------------------------------------------------------
// AuditLog
// ---------------------------------------------------------------------------

/// Middleware that logs tool invocations at `info!` level.
///
/// Does not modify any tools or results -- purely observational.
#[derive(Debug, Clone, Default)]
pub struct AuditLog;

#[async_trait]
impl Middleware for AuditLog {
    async fn before_call(&self, request: ToolCallRequest) -> Result<ToolCallRequest, ToolError> {
        let args_summary = summarize_json(&request.args, 200);
        info!(
            tool = %request.name,
            args = %args_summary,
            "tool call started"
        );
        Ok(request)
    }

    async fn after_call(
        &self,
        request: &ToolCallRequest,
        result: CallToolResult,
    ) -> Result<CallToolResult, ToolError> {
        let content_len: usize = result
            .content
            .iter()
            .map(|b| match b {
                ContentBlock::Text { text } => text.len(),
            })
            .sum();

        info!(
            tool = %request.name,
            is_error = result.is_error,
            content_bytes = content_len,
            "tool call completed"
        );
        Ok(result)
    }
}

/// Produce a compact JSON summary, truncating if necessary.
fn summarize_json(value: &serde_json::Value, max_len: usize) -> String {
    let s = value.to_string();
    if s.len() <= max_len {
        s
    } else {
        let mut end = max_len;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &s[..end])
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a `ToolCallRequest`.
    fn make_request(name: &str, args: serde_json::Value) -> ToolCallRequest {
        ToolCallRequest {
            name: name.to_string(),
            args,
        }
    }

    /// Helper: build sample `ToolDefinition` list.
    fn sample_tools() -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "echo".into(),
                description: "Echoes input".into(),
                input_schema: serde_json::json!({"type": "object"}),
            },
            ToolDefinition {
                name: "exec_shell".into(),
                description: "Executes shell commands".into(),
                input_schema: serde_json::json!({"type": "object"}),
            },
            ToolDefinition {
                name: "web_fetch".into(),
                description: "Fetches a URL".into(),
                input_schema: serde_json::json!({"type": "object"}),
            },
        ]
    }

    // ── SecurityGuard ────────────────────────────────────────────────

    #[tokio::test]
    async fn security_guard_rejects_disallowed_shell_command() {
        let guard = SecurityGuard::default();
        let req = make_request(
            "exec_shell",
            serde_json::json!({"command": "curl http://evil.com"}),
        );
        let result = guard.before_call(req).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::PermissionDenied { tool, reason } => {
                assert_eq!(tool, "exec_shell");
                assert!(reason.contains("command rejected"));
                assert!(reason.contains("not allowed"));
            }
            other => panic!("expected PermissionDenied, got: {other}"),
        }
    }

    #[tokio::test]
    async fn security_guard_rejects_dangerous_command() {
        let guard = SecurityGuard::default();
        let req = make_request(
            "exec_shell",
            serde_json::json!({"command": "sudo rm -rf /"}),
        );
        let result = guard.before_call(req).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::PermissionDenied { tool, reason } => {
                assert_eq!(tool, "exec_shell");
                assert!(reason.contains("dangerous pattern"));
            }
            other => panic!("expected PermissionDenied, got: {other}"),
        }
    }

    #[tokio::test]
    async fn security_guard_allows_safe_command() {
        let guard = SecurityGuard::default();
        let req = make_request(
            "exec_shell",
            serde_json::json!({"command": "echo hello world"}),
        );
        let result = guard.before_call(req).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "exec_shell");
    }

    #[tokio::test]
    async fn security_guard_rejects_private_url() {
        let guard = SecurityGuard::default();
        let req = make_request(
            "web_fetch",
            serde_json::json!({"url": "http://192.168.1.1/admin"}),
        );
        let result = guard.before_call(req).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::PermissionDenied { tool, reason } => {
                assert_eq!(tool, "web_fetch");
                assert!(reason.contains("URL rejected"));
            }
            other => panic!("expected PermissionDenied, got: {other}"),
        }
    }

    #[tokio::test]
    async fn security_guard_allows_public_url() {
        let guard = SecurityGuard::default();
        let req = make_request(
            "web_fetch",
            serde_json::json!({"url": "https://example.com/api"}),
        );
        let result = guard.before_call(req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn security_guard_ignores_unrelated_tools() {
        let guard = SecurityGuard::default();
        let req = make_request("echo", serde_json::json!({"text": "hello"}));
        let result = guard.before_call(req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn security_guard_rejects_metadata_endpoint() {
        let guard = SecurityGuard::default();
        let req = make_request(
            "web_search",
            serde_json::json!({"url": "http://169.254.169.254/latest/meta-data/"}),
        );
        let result = guard.before_call(req).await;
        assert!(result.is_err());
    }

    // ── PermissionFilter ─────────────────────────────────────────────

    #[tokio::test]
    async fn permission_filter_with_allowlist_strips_unauthorized() {
        let filter = PermissionFilter::new(Some(vec!["echo".into()]));
        let tools = filter.filter_tools(sample_tools()).await;
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "echo");
    }

    #[tokio::test]
    async fn permission_filter_with_none_allows_all() {
        let filter = PermissionFilter::new(None);
        let tools = filter.filter_tools(sample_tools()).await;
        assert_eq!(tools.len(), 3);
    }

    #[tokio::test]
    async fn permission_filter_empty_allowlist_strips_all() {
        let filter = PermissionFilter::new(Some(vec![]));
        let tools = filter.filter_tools(sample_tools()).await;
        assert!(tools.is_empty());
    }

    #[tokio::test]
    async fn permission_filter_multiple_allowed() {
        let filter = PermissionFilter::new(Some(vec!["echo".into(), "web_fetch".into()]));
        let tools = filter.filter_tools(sample_tools()).await;
        assert_eq!(tools.len(), 2);
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"echo"));
        assert!(names.contains(&"web_fetch"));
    }

    // ── ResultGuard ──────────────────────────────────────────────────

    #[tokio::test]
    async fn result_guard_truncates_oversized_content() {
        let guard = ResultGuard::new(50);
        let result = CallToolResult::text("x".repeat(200));
        let req = make_request("test", serde_json::json!({}));
        let guarded = guard.after_call(&req, result).await.unwrap();

        match &guarded.content[0] {
            ContentBlock::Text { text } => {
                assert!(text.ends_with("\n[truncated]"));
                // The body before the sentinel should be at most max_bytes.
                let body = text.strip_suffix("\n[truncated]").unwrap();
                assert!(body.len() <= 50);
            }
        }
    }

    #[tokio::test]
    async fn result_guard_passes_short_content_unchanged() {
        let guard = ResultGuard::new(1024);
        let original = CallToolResult::text("short output");
        let req = make_request("test", serde_json::json!({}));
        let guarded = guard.after_call(&req, original.clone()).await.unwrap();
        assert_eq!(guarded, original);
    }

    #[tokio::test]
    async fn result_guard_preserves_is_error_flag() {
        let guard = ResultGuard::new(10);
        let result = CallToolResult::error("a very long error message that exceeds the limit");
        let req = make_request("test", serde_json::json!({}));
        let guarded = guard.after_call(&req, result).await.unwrap();
        assert!(guarded.is_error);
    }

    // ── AuditLog ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn audit_log_passthrough_before_call() {
        let audit = AuditLog;
        let req = make_request("echo", serde_json::json!({"text": "hello"}));
        let result = audit.before_call(req).await.unwrap();
        assert_eq!(result.name, "echo");
        assert_eq!(result.args["text"], "hello");
    }

    #[tokio::test]
    async fn audit_log_passthrough_after_call() {
        let audit = AuditLog;
        let original = CallToolResult::text("output");
        let req = make_request("echo", serde_json::json!({}));
        let result = audit.after_call(&req, original.clone()).await.unwrap();
        assert_eq!(result, original);
    }

    #[tokio::test]
    async fn audit_log_does_not_modify_tools() {
        let audit = AuditLog;
        let tools = sample_tools();
        let filtered = audit.filter_tools(tools.clone()).await;
        assert_eq!(filtered.len(), tools.len());
        for (a, b) in filtered.iter().zip(tools.iter()) {
            assert_eq!(a.name, b.name);
        }
    }

    // ── Helper functions ─────────────────────────────────────────────

    #[test]
    fn extract_host_simple_https() {
        assert_eq!(
            extract_host("https://example.com/path"),
            Some("example.com".to_string())
        );
    }

    #[test]
    fn extract_host_with_port() {
        assert_eq!(
            extract_host("http://localhost:8080/api"),
            Some("localhost".to_string())
        );
    }

    #[test]
    fn extract_host_ip_address() {
        assert_eq!(
            extract_host("http://192.168.1.1/admin"),
            Some("192.168.1.1".to_string())
        );
    }

    #[test]
    fn extract_host_ipv6_bracket() {
        assert_eq!(extract_host("http://[::1]:8080/"), Some("::1".to_string()));
    }

    #[test]
    fn extract_host_with_userinfo() {
        assert_eq!(
            extract_host("http://user:pass@example.com/"),
            Some("example.com".to_string())
        );
    }

    #[test]
    fn extract_host_no_scheme() {
        // Best-effort: treats entire string as authority.
        assert_eq!(extract_host("example.com"), Some("example.com".to_string()));
    }

    #[test]
    fn command_policy_validate_allowed() {
        let policy = CommandPolicy::default();
        assert!(policy.validate("echo hello").is_ok());
        assert!(policy.validate("ls -la").is_ok());
        assert!(policy.validate("cat file.txt").is_ok());
    }

    #[test]
    fn command_policy_validate_rejected() {
        let policy = CommandPolicy::default();
        assert!(policy.validate("curl http://evil.com").is_err());
        assert!(policy.validate("python3 -c 'evil'").is_err());
    }

    #[test]
    fn command_policy_validate_dangerous() {
        let policy = CommandPolicy::default();
        let result = policy.validate("echo; sudo rm -rf /");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("dangerous pattern"));
    }

    #[test]
    fn url_policy_validate_public() {
        let policy = UrlPolicy::default();
        assert!(policy.validate("https://example.com").is_ok());
    }

    #[test]
    fn url_policy_validate_private_blocked() {
        let policy = UrlPolicy::default();
        assert!(policy.validate("http://192.168.1.1").is_err());
        assert!(policy.validate("http://10.0.0.1").is_err());
        assert!(policy.validate("http://127.0.0.1").is_err());
    }

    #[test]
    fn url_policy_validate_disabled() {
        let policy = UrlPolicy {
            enabled: false,
            blocked_domains: HashSet::new(),
        };
        assert!(policy.validate("http://127.0.0.1").is_ok());
    }

    #[test]
    fn summarize_json_short() {
        let val = serde_json::json!({"key": "value"});
        let s = summarize_json(&val, 200);
        assert_eq!(s, r#"{"key":"value"}"#);
    }

    #[test]
    fn summarize_json_truncated() {
        let val = serde_json::json!({"key": "a".repeat(300)});
        let s = summarize_json(&val, 50);
        assert!(s.len() <= 53); // 50 + "..."
        assert!(s.ends_with("..."));
    }

    // ── is_private_or_loopback (A6) ─────────────────────────────────

    #[test]
    fn private_ip_rfc1918_10_range() {
        assert!(is_private_or_loopback("10.0.0.1"));
        assert!(is_private_or_loopback("10.255.255.255"));
    }

    #[test]
    fn private_ip_rfc1918_172_full_range() {
        // Full /12 range: 172.16.0.0 through 172.31.255.255
        assert!(is_private_or_loopback("172.16.0.1"));
        assert!(is_private_or_loopback("172.20.0.1"));
        assert!(is_private_or_loopback("172.30.0.1"));
        assert!(is_private_or_loopback("172.31.255.1"));
        // Outside the /12 range
        assert!(!is_private_or_loopback("172.15.255.255"));
        assert!(!is_private_or_loopback("172.32.0.1"));
    }

    #[test]
    fn private_ip_rfc1918_192_168_range() {
        assert!(is_private_or_loopback("192.168.0.1"));
        assert!(is_private_or_loopback("192.168.255.255"));
        assert!(!is_private_or_loopback("192.167.0.1"));
    }

    #[test]
    fn private_ip_loopback_range() {
        assert!(is_private_or_loopback("127.0.0.1"));
        assert!(is_private_or_loopback("127.255.255.255"));
    }

    #[test]
    fn private_ip_link_local() {
        assert!(is_private_or_loopback("169.254.1.1"));
        assert!(is_private_or_loopback("169.254.169.254"));
    }

    #[test]
    fn private_ip_special_addresses() {
        assert!(is_private_or_loopback("0.0.0.0"));
        assert!(is_private_or_loopback("localhost"));
        assert!(is_private_or_loopback("::1"));
    }

    #[test]
    fn private_ip_ipv4_mapped_ipv6() {
        assert!(is_private_or_loopback("::ffff:10.0.0.1"));
        assert!(is_private_or_loopback("::ffff:192.168.1.1"));
        assert!(is_private_or_loopback("::ffff:172.30.0.1"));
        assert!(is_private_or_loopback("::ffff:127.0.0.1"));
        // Public via mapped should be allowed
        assert!(!is_private_or_loopback("::ffff:8.8.8.8"));
    }

    #[test]
    fn public_ips_allowed() {
        assert!(!is_private_or_loopback("8.8.8.8"));
        assert!(!is_private_or_loopback("1.1.1.1"));
        assert!(!is_private_or_loopback("172.32.0.1"));
        assert!(!is_private_or_loopback("example.com"));
    }

    // ── UrlPolicy with full SSRF coverage (A6) ─────────────────────

    #[test]
    fn url_policy_blocks_full_172_range() {
        let policy = UrlPolicy::default();
        assert!(policy.validate("http://172.16.0.1").is_err());
        assert!(policy.validate("http://172.20.0.1").is_err());
        assert!(policy.validate("http://172.30.0.1").is_err());
        assert!(policy.validate("http://172.31.255.1").is_err());
        // Outside range
        assert!(policy.validate("http://172.15.255.255").is_ok());
        assert!(policy.validate("http://172.32.0.1").is_ok());
    }

    #[test]
    fn url_policy_blocks_ipv4_mapped_ipv6() {
        let policy = UrlPolicy::default();
        assert!(policy.validate("http://[::ffff:10.0.0.1]/").is_err());
        assert!(policy.validate("http://[::ffff:192.168.1.1]/").is_err());
        assert!(policy.validate("http://[::ffff:172.30.0.1]/").is_err());
    }

    #[test]
    fn url_policy_blocks_ipv6_loopback() {
        let policy = UrlPolicy::default();
        assert!(policy.validate("http://[::1]/").is_err());
        assert!(policy.validate("http://[::1]:8080/").is_err());
    }

    #[test]
    fn url_policy_blocks_link_local() {
        let policy = UrlPolicy::default();
        assert!(policy.validate("http://169.254.1.1").is_err());
        assert!(policy.validate("http://169.254.169.254").is_err());
    }

    #[tokio::test]
    async fn security_guard_rejects_172_30_url() {
        let guard = SecurityGuard::default();
        let req = make_request(
            "web_fetch",
            serde_json::json!({"url": "http://172.30.0.1/admin"}),
        );
        let result = guard.before_call(req).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::PermissionDenied { reason, .. } => {
                assert!(reason.contains("URL rejected"));
            }
            other => panic!("expected PermissionDenied, got: {other}"),
        }
    }
}
