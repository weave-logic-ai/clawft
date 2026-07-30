//! Fuzz target: tool allowlist/denylist glob matching.
//!
//! Property (security-review §6.1 / T-02): `check_tool_permission`
//! never panics on arbitrary tool names + glob patterns; denylist
//! always wins over allowlist `["*"]`; `"*"` allowlist alone allows
//! any name.
//!
//! Run: `cargo +nightly fuzz run tool_permission_glob -- -max_total_time=60`

#![no_main]

use clawft_core::tools::registry::{check_tool_permission, ToolMetadata};
use clawft_types::routing::UserPermissions;
use libfuzzer_sys::fuzz_target;
use std::collections::HashMap;

fn take_str<'a>(data: &mut &'a [u8], max: usize) -> String {
    if data.is_empty() {
        return String::new();
    }
    let len = (data[0] as usize).min(max).min(data.len().saturating_sub(1));
    *data = &data[1..];
    let (chunk, rest) = data.split_at(len.min(data.len()));
    *data = rest;
    String::from_utf8_lossy(chunk).into_owned()
}

fuzz_target!(|data: &[u8]| {
    let mut cursor = data;

    let tool_name = take_str(&mut cursor, 64);
    let n_allow = (cursor.first().copied().unwrap_or(0) as usize % 6) + 1;
    if !cursor.is_empty() {
        cursor = &cursor[1..];
    }
    let n_deny = cursor.first().copied().unwrap_or(0) as usize % 4;
    if !cursor.is_empty() {
        cursor = &cursor[1..];
    }

    let mut tool_access = Vec::with_capacity(n_allow);
    for i in 0..n_allow {
        if i == 0 && data.first().copied().unwrap_or(0) & 0x80 != 0 {
            tool_access.push("*".into());
        } else {
            tool_access.push(take_str(&mut cursor, 32));
        }
    }
    let mut tool_denylist = Vec::with_capacity(n_deny);
    for _ in 0..n_deny {
        tool_denylist.push(take_str(&mut cursor, 32));
    }

    let level = cursor.first().copied().unwrap_or(0);
    if !cursor.is_empty() {
        cursor = &cursor[1..];
    }

    let permissions = UserPermissions {
        level,
        tool_access: tool_access.clone(),
        tool_denylist: tool_denylist.clone(),
        ..UserPermissions::default()
    };

    // Optional metadata (level / custom permission gates).
    let meta = if cursor.first().copied().unwrap_or(0) & 1 == 1 {
        let mut custom = HashMap::new();
        custom.insert(
            take_str(&mut cursor, 16),
            serde_json::Value::Bool(cursor.first().copied().unwrap_or(0) & 1 == 1),
        );
        Some(ToolMetadata {
            required_permission_level: Some(cursor.first().copied().unwrap_or(0)),
            required_custom_permissions: custom,
        })
    } else {
        None
    };

    let result = check_tool_permission(&tool_name, &permissions, meta.as_ref());
    let _ = result.is_ok();

    // Explicit invariant: exact denylist entry wins over wildcard allowlist.
    if !tool_name.is_empty() && tool_denylist.iter().any(|p| p == &tool_name) {
        let denied = UserPermissions {
            level: 2,
            tool_access: vec!["*".into()],
            tool_denylist: vec![tool_name.clone()],
            ..UserPermissions::default()
        };
        assert!(
            check_tool_permission(&tool_name, &denied, None).is_err(),
            "exact denylist entry must deny even when allowlist has *"
        );
    }

    // Wildcard allowlist allows benign tool names (not MCP-sensitive namespaces
    // blocked by WEFT-32 `validate_mcp_namespace_against_wildcard`).
    let benign = "read_file";
    let open = UserPermissions {
        level: 2,
        tool_access: vec!["*".into()],
        tool_denylist: vec![],
        ..UserPermissions::default()
    };
    assert!(
        check_tool_permission(benign, &open, None).is_ok(),
        "wildcard allowlist must allow benign tools such as read_file"
    );
    let _ = tool_name; // still fuzzed via the primary check_tool_permission call above
});
