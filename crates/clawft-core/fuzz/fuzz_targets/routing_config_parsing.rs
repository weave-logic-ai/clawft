//! Fuzz target: `RoutingConfig` JSON deserialization + semantic validation.
//!
//! Attack surface (security-review §6.1): random JSON blobs must never
//! panic. Output is always a successful `RoutingConfig` or a serde error;
//! post-deser validation must also never panic.
//!
//! Run: `cargo +nightly fuzz run routing_config_parsing -- -max_total_time=60`

#![no_main]

use clawft_core::routing_validation::validate_routing_config;
use clawft_types::routing::RoutingConfig;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Prefer UTF-8 JSON; fall back to lossy so every byte sequence is exercised.
    let text = match std::str::from_utf8(data) {
        Ok(s) => s.to_string(),
        Err(_) => String::from_utf8_lossy(data).into_owned(),
    };

    if let Ok(config) = serde_json::from_str::<RoutingConfig>(&text) {
        // Semantic validation: collect diagnostics; never panic.
        let _errors = validate_routing_config(&config);
    }
});
