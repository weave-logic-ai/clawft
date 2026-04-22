//! Sanity test: every tray-chip detail surface fixture parses, and
//! running the composer against a representative substrate snapshot
//! doesn't panic. The composer path is the production rendering
//! mode; a parse regression would silently demote to the raw-JSON
//! fallback, so we catch it here instead.

#![cfg(not(target_arch = "wasm32"))]

use clawft_gui_egui::surface_host::render_headless;
use clawft_surface::parse::parse_surface_toml;
use clawft_surface::substrate::OntologySnapshot;
use serde_json::json;

const CHIP_FIXTURES: &[(&str, &str)] = &[
    (
        "kernel",
        include_str!("../../clawft-surface/fixtures/weftos-chip-kernel.toml"),
    ),
    (
        "mesh",
        include_str!("../../clawft-surface/fixtures/weftos-chip-mesh.toml"),
    ),
    (
        "exochain",
        include_str!("../../clawft-surface/fixtures/weftos-chip-exochain.toml"),
    ),
    (
        "wifi",
        include_str!("../../clawft-surface/fixtures/weftos-chip-wifi.toml"),
    ),
    (
        "bluetooth",
        include_str!("../../clawft-surface/fixtures/weftos-chip-bluetooth.toml"),
    ),
    (
        "audio",
        include_str!("../../clawft-surface/fixtures/weftos-chip-audio.toml"),
    ),
    (
        "tof",
        include_str!("../../clawft-surface/fixtures/weftos-chip-tof.toml"),
    ),
];

fn populated_snapshot() -> OntologySnapshot {
    let mut s = OntologySnapshot::empty();
    s.put(
        "substrate/kernel/status",
        json!({
            "state": "running",
            "uptime_secs": 123.4,
            "process_count": 4,
            "service_count": 3,
        }),
    );
    s.put(
        "substrate/mesh/status",
        json!({
            "total_nodes": 3,
            "healthy_nodes": 3,
            "consensus_enabled": true,
            "total_shards": 16,
            "active_shards": 12,
        }),
    );
    s.put(
        "substrate/chain/status",
        json!({
            "chain_id": "weftos.local",
            "sequence": 42,
            "event_count": 1000,
            "checkpoint_count": 3,
            "last_hash": "abc123",
        }),
    );
    s.put(
        "substrate/network/wifi",
        json!({"state": "connected", "iface": "wlan0"}),
    );
    s.put(
        "substrate/bluetooth",
        json!({"present": true, "enabled": true, "controller": "hci0"}),
    );
    s.put(
        "substrate/sensor/mic",
        json!({
            "available": true,
            "rms_db": -18.0,
            "peak_db": -6.0,
            "sample_rate": 16_000,
            "samples_in_window": 8000,
            "characterization": "rate",
        }),
    );
    // ToF test frame: an 8×8 with a few non-0xFFFF values so the
    // tray chip reads as On and the fixture can render min/max.
    let mut depths = vec![65535u16; 64];
    for (i, d) in depths.iter_mut().enumerate().take(32) {
        *d = 300 + (i as u16 * 30);
    }
    s.put(
        "substrate/sensor/tof",
        json!({
            "available": true,
            "width": 8,
            "height": 8,
            "depths_mm": depths,
            "min_mm": 300,
            "max_mm": 1230,
            "frame_count": 1,
        }),
    );
    s
}

#[test]
fn every_chip_fixture_parses() {
    for (name, toml) in CHIP_FIXTURES {
        parse_surface_toml(toml)
            .unwrap_or_else(|e| panic!("chip surface `{name}` failed to parse: {e}"));
    }
}

#[test]
fn every_chip_fixture_composes_populated_snapshot() {
    for (name, toml) in CHIP_FIXTURES {
        let tree = parse_surface_toml(toml)
            .unwrap_or_else(|e| panic!("chip surface `{name}` failed to parse: {e}"));
        let responses = render_headless(&tree, populated_snapshot());
        // Every fixture must emit at least one primitive response —
        // an empty outcome would mean the composer skipped the whole
        // tree (e.g. unsupported primitive, silent eval failure).
        assert!(
            !responses.is_empty(),
            "chip surface `{name}` rendered no primitives"
        );
    }
}

#[test]
fn every_chip_fixture_composes_empty_snapshot() {
    // No panic when bindings resolve to Null (daemon offline, wasm
    // pre-bridge, etc.) — the chip window still needs to draw.
    for (name, toml) in CHIP_FIXTURES {
        let tree = parse_surface_toml(toml)
            .unwrap_or_else(|e| panic!("chip surface `{name}` failed to parse: {e}"));
        let _ = render_headless(&tree, OntologySnapshot::empty());
    }
}
