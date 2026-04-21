//! M1.5 integration test 3: headless composer render.
//!
//! Loads the admin-panel TOML + a synthetic kernel snapshot, runs
//! the composer through one egui pass, and asserts on the emitted
//! `CanonResponse` stream.

use clawft_surface::parse::parse_surface_toml;
use clawft_surface::substrate::OntologySnapshot;
use clawft_surface::test_harness::render_headless;
use serde_json::json;

const FIXTURE: &str = include_str!("../fixtures/weftos-admin-desktop.toml");

fn healthy_snapshot() -> OntologySnapshot {
    let mut s = OntologySnapshot::empty();
    s.put(
        "substrate/kernel/status",
        json!({"state": "healthy", "uptime_ms": 3_600_000u64}),
    );
    s.put(
        "substrate/kernel/services",
        json!([
            {"name": "mesh-listener", "status": "healthy", "cpu_percent": 42.0},
            {"name": "rpc-gateway",   "status": "healthy", "cpu_percent": 9.0},
            {"name": "audit-sink",    "status": "at_risk", "cpu_percent": 88.0},
        ]),
    );
    s.put(
        "substrate/kernel/processes",
        json!([
            {"pid": 101, "name": "weaver", "cpu": 4.2},
            {"pid": 222, "name": "claude", "cpu": 21.7},
        ]),
    );
    s.put(
        "substrate/kernel/logs",
        json!(["[t+0s] boot ok", "[t+1s] 3 services ready"]),
    );
    s
}

#[test]
fn admin_panel_renders_without_panic() {
    let tree = parse_surface_toml(FIXTURE).expect("parse");
    let snap = healthy_snapshot();
    let responses = render_headless(&tree, snap);
    assert!(
        !responses.is_empty(),
        "composer must emit at least one response for a non-empty surface"
    );
}

#[test]
fn admin_panel_emits_expected_primitive_counts() {
    let tree = parse_surface_toml(FIXTURE).expect("parse");
    let snap = healthy_snapshot();
    let responses = render_headless(&tree, snap);

    let count = |iri: &str| responses.iter().filter(|r| r.identity == iri).count();
    assert_eq!(count("ui://grid"), 1, "one top-level grid");
    assert_eq!(count("ui://chip"), 2, "two overview chips");
    assert_eq!(count("ui://table"), 1, "one process table");
    assert_eq!(count("ui://gauge"), 1, "one mesh-listener gauge");
    assert_eq!(count("ui://stream-view"), 1, "one log stream");
    assert_eq!(count("ui://stack"), 2, "overview stack + services stack");
}

#[test]
fn affordance_kill_is_declared_on_process_table() {
    // Governance intersection is stubbed in M1.5, so every declared
    // affordance survives through the tree to the composer boundary.
    let tree = parse_surface_toml(FIXTURE).expect("parse");
    assert!(tree.any_affordance_with_verb("rpc.kernel.kill"));
}
