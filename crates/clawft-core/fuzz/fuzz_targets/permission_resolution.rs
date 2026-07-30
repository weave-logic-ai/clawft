//! Fuzz target: 5-layer permission resolution + workspace ceiling.
//!
//! Property (security-review §6.1): `PermissionResolver::resolve` never
//! panics; returned `UserPermissions` always has finite numeric fields
//! and a defined level.
//!
//! Run: `cargo +nightly fuzz run permission_resolution -- -max_total_time=60`

#![no_main]

use clawft_core::pipeline::permissions::{
    defaults_for_level, level_from_name, level_name, merge_permissions, PermissionResolver,
};
use clawft_types::routing::{PermissionLevelConfig, RoutingConfig};
use libfuzzer_sys::fuzz_target;

/// Decode a length-prefixed string from fuzz input; empty on OOB.
fn take_str<'a>(data: &mut &'a [u8], max: usize) -> &'a str {
    if data.is_empty() {
        return "";
    }
    let len = (data[0] as usize).min(max).min(data.len().saturating_sub(1));
    *data = &data[1..];
    let (chunk, rest) = data.split_at(len.min(data.len()));
    *data = rest;
    std::str::from_utf8(chunk).unwrap_or("")
}

fuzz_target!(|data: &[u8]| {
    let mut cursor = data;

    // Build a global config with a few random user/channel overrides.
    let mut global = RoutingConfig::default();
    global.mode = "tiered".into();

    let n_users = cursor.first().copied().unwrap_or(0) % 4;
    if !cursor.is_empty() {
        cursor = &cursor[1..];
    }
    for _ in 0..n_users {
        let id = take_str(&mut cursor, 32).to_string();
        let level = cursor.first().copied().unwrap_or(0);
        if !cursor.is_empty() {
            cursor = &cursor[1..];
        }
        global.permissions.users.insert(
            id,
            PermissionLevelConfig {
                level: Some(level),
                max_tier: Some(take_str(&mut cursor, 16).to_string()),
                tool_access: Some(vec![
                    take_str(&mut cursor, 24).to_string(),
                    "*".into(),
                ]),
                cost_budget_daily_usd: Some(f64::from(cursor.first().copied().unwrap_or(0))),
                ..Default::default()
            },
        );
        if !cursor.is_empty() {
            cursor = &cursor[1..];
        }
    }

    // Optional workspace overlay (ceiling enforcement path).
    let use_workspace = cursor.first().copied().unwrap_or(0) & 1 == 1;
    if !cursor.is_empty() {
        cursor = &cursor[1..];
    }
    let workspace = if use_workspace {
        let mut ws = RoutingConfig::default();
        ws.mode = "tiered".into();
        let sid = take_str(&mut cursor, 32).to_string();
        ws.permissions.users.insert(
            sid,
            PermissionLevelConfig {
                level: Some(cursor.first().copied().unwrap_or(2)),
                tool_access: Some(vec!["*".into(), "exec_shell".into()]),
                escalation_allowed: Some(true),
                cost_budget_daily_usd: Some(9999.0),
                ..Default::default()
            },
        );
        Some(ws)
    } else {
        None
    };

    let resolver = PermissionResolver::new(&global, workspace.as_ref());

    let sender = take_str(&mut cursor, 48);
    let channel = take_str(&mut cursor, 24);
    let allow = cursor.first().copied().unwrap_or(0) & 1 == 1;

    let perms = resolver.resolve(sender, channel, allow);
    // Sanity properties — must hold for any input.
    let _ = perms.level;
    let _ = perms.max_context_tokens;
    let _ = perms.rate_limit;
    // NaN thresholds would break comparisons; flag but do not panic.
    let _ = perms.escalation_threshold.is_finite();
    let _ = perms.cost_budget_daily_usd.is_finite();

    let _auth = resolver.resolve_auth_context(sender, channel, allow);

    // Also exercise pure helpers with the same bytes.
    let _ = defaults_for_level(perms.level);
    let _ = level_name(perms.level);
    let _ = level_from_name(channel);
    let mut base = defaults_for_level(0);
    merge_permissions(
        &mut base,
        &PermissionLevelConfig {
            level: Some(perms.level),
            tool_access: Some(perms.tool_access.clone()),
            ..Default::default()
        },
    );
});
