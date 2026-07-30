//! Fuzz target: complexity-based tier escalation chain.
//!
//! Property (security-review §2.3): `TieredRouter::route` never panics
//! for arbitrary complexity / escalation config / permission combos;
//! empty-tier and max_escalation_tiers edge cases are handled.
//!
//! Run: `cargo +nightly fuzz run escalation_chain -- -max_total_time=60`

#![no_main]

use clawft_core::pipeline::tiered_router::TieredRouter;
use clawft_core::pipeline::traits::{ChatRequest, ModelRouter, TaskProfile, TaskType};
use clawft_types::routing::{
    AuthContext, EscalationConfig, ModelTierConfig, RoutingConfig, UserPermissions,
};
use futures::executor::block_on;
use libfuzzer_sys::fuzz_target;

fn complexity_from(b: u8) -> f32 {
    // Map 0..=255 into [0.0, 1.0]; also inject a few non-finite values
    // when high bits are set so comparison paths are exercised.
    match b {
        0xff => f32::NAN,
        0xfe => f32::INFINITY,
        0xfd => f32::NEG_INFINITY,
        v => f32::from(v) / 255.0,
    }
}

fuzz_target!(|data: &[u8]| {
    let n_tiers = (data.first().copied().unwrap_or(2) as usize % 5) + 1;
    let mut tiers = Vec::with_capacity(n_tiers);
    for i in 0..n_tiers {
        let b = data.get(1 + i).copied().unwrap_or(i as u8);
        let lo = f32::from(b) / 255.0;
        let hi = (lo + 0.25).min(1.0);
        tiers.push(ModelTierConfig {
            name: match i {
                0 => "free".into(),
                1 => "standard".into(),
                2 => "premium".into(),
                3 => "elite".into(),
                _ => format!("tier{i}"),
            },
            models: vec![format!("provider/model-{i}")],
            complexity_range: [lo, hi],
            cost_per_1k_tokens: f64::from(b) * 0.001,
            max_context_tokens: 1024 + (b as usize) * 64,
        });
    }

    let esc_enabled = data.get(8).copied().unwrap_or(1) & 1 == 1;
    let max_esc = u32::from(data.get(9).copied().unwrap_or(1) % 4);
    let threshold = complexity_from(data.get(10).copied().unwrap_or(128));

    let config = RoutingConfig {
        mode: "tiered".into(),
        tiers,
        escalation: EscalationConfig {
            enabled: esc_enabled,
            threshold,
            max_escalation_tiers: max_esc,
        },
        fallback_model: if data.get(11).copied().unwrap_or(0) & 1 == 1 {
            Some("fallback/model".into())
        } else {
            None
        },
        ..RoutingConfig::default()
    };

    // Empty-tiers panic guard: also construct with zero tiers once in a while.
    let router = if data.get(12).copied().unwrap_or(1) == 0 {
        TieredRouter::new(RoutingConfig {
            mode: "tiered".into(),
            tiers: vec![],
            ..RoutingConfig::default()
        })
    } else {
        TieredRouter::new(config)
    };

    let max_tier = match data.get(13).copied().unwrap_or(0) % 4 {
        0 => "free",
        1 => "standard",
        2 => "premium",
        _ => "elite",
    };
    let perms = UserPermissions {
        level: data.get(14).copied().unwrap_or(1) % 3,
        max_tier: max_tier.into(),
        escalation_allowed: data.get(15).copied().unwrap_or(1) & 1 == 1,
        escalation_threshold: complexity_from(data.get(16).copied().unwrap_or(100)),
        model_override: data.get(17).copied().unwrap_or(0) & 1 == 1,
        rate_limit: 0, // unlimited so rate limiter (absent) is not a factor
        tool_access: vec!["*".into()],
        ..UserPermissions::default()
    };

    let auth = AuthContext {
        sender_id: "fuzz-user".into(),
        channel: "fuzz".into(),
        permissions: perms,
    };

    let req = ChatRequest {
        messages: vec![],
        tools: vec![],
        model: if auth.permissions.model_override {
            Some("override/model".into())
        } else {
            None
        },
        max_tokens: None,
        temperature: None,
        auth_context: Some(auth),
        complexity_boost: complexity_from(data.get(18).copied().unwrap_or(0)),
        tool_choice: None,
    };

    let profile = TaskProfile {
        task_type: TaskType::Unknown,
        complexity: complexity_from(data.get(19).copied().unwrap_or(200)),
        keywords: vec![],
    };

    // Must never panic.
    let decision = block_on(router.route(&req, &profile));
    let _ = (
        decision.provider,
        decision.model,
        decision.escalated,
        decision.budget_constrained,
        decision.tier,
    );
});
