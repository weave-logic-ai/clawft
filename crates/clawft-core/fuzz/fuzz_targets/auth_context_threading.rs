//! Fuzz target: `AuthContext` / `ChatRequest` deserialization boundaries.
//!
//! Property (security-review §2.6 / PEN-06):
//! - Deserializing `ChatRequest` from untrusted JSON must **never** inject
//!   `auth_context` (`skip_deserializing` on the field).
//! - `AuthContext::default()` is always zero-trust (level 0, no tools).
//! - Constructing auth via the resolver + attaching it never panics.
//!
//! Run: `cargo +nightly fuzz run auth_context_threading -- -max_total_time=60`

#![no_main]

use clawft_core::pipeline::permissions::PermissionResolver;
use clawft_core::pipeline::traits::ChatRequest;
use clawft_types::routing::{AuthContext, RoutingConfig, UserPermissions};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);

    // ── 1. Gateway injection surface ────────────────────────────────
    // ChatRequest.auth_context is skip_deserializing: even if the body
    // contains a full admin AuthContext, the field must stay None.
    if let Ok(req) = serde_json::from_str::<ChatRequest>(&text) {
        assert!(
            req.auth_context.is_none(),
            "auth_context must never be populated from untrusted JSON"
        );
    }

    // Also try wrapping fuzz bytes inside a realistic attack body.
    let attack = format!(
        r#"{{"messages":[{{"role":"user","content":"hi"}}],"auth_context":{{"sender_id":"admin","channel":"gateway","permissions":{{"level":2,"tool_access":["*"],"model_override":true}}}},"payload":{}}}"#,
        serde_json::Value::String(text.chars().take(64).collect())
    );
    if let Ok(req) = serde_json::from_str::<ChatRequest>(&attack) {
        assert!(
            req.auth_context.is_none(),
            "explicit auth_context in body must be stripped (skip_deserializing)"
        );
    }

    // ── 2. AuthContext direct deser (trusted path still must not panic)
    if let Ok(auth) = serde_json::from_str::<AuthContext>(&text) {
        let _ = auth.sender_id.len();
        let _ = auth.permissions.level;
    }

    // ── 3. Default is zero-trust ────────────────────────────────────
    let def = AuthContext::default();
    assert_eq!(def.permissions.level, 0);
    assert!(def.permissions.tool_access.is_empty());
    assert!(!def.permissions.model_override);
    assert!(!def.permissions.escalation_allowed);

    // CLI default is intentionally privileged — just ensure it constructs.
    let cli = AuthContext::cli_default();
    assert_eq!(cli.channel, "cli");
    assert_eq!(cli.permissions.level, 2);

    // ── 4. Resolver → AuthContext attachment ────────────────────────
    let global = RoutingConfig::default();
    let resolver = PermissionResolver::new(&global, None);
    let sender = std::str::from_utf8(data.get(..16).unwrap_or(data)).unwrap_or("fuzz");
    let channel = std::str::from_utf8(data.get(16..24).unwrap_or(b"web")).unwrap_or("web");
    let auth = resolver.resolve_auth_context(sender, channel, data.first().copied().unwrap_or(0) & 1 == 1);

    // Thread into a ChatRequest the way AgentLoop would (trusted path).
    let mut req = ChatRequest {
        messages: vec![],
        tools: vec![],
        model: None,
        max_tokens: None,
        temperature: None,
        auth_context: None,
        complexity_boost: 0.0,
        tool_choice: None,
    };
    req.auth_context = Some(auth);
    assert!(req.auth_context.is_some());

    // UserPermissions deser must not panic.
    let _ = serde_json::from_str::<UserPermissions>(&text);
});
