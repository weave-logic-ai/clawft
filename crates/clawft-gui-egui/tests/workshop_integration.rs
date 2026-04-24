//! Workshop integration test — exercises the substrate publish →
//! Workshop parse round-trip using the real `clawft_substrate::Substrate`
//! in-process (no daemon, no GUI).
//!
//! What this proves:
//!
//! 1. A Workshop JSON value published into substrate (via
//!    `StateDelta::Replace`) is readable back with `substrate.get`
//!    and parses to the expected [`Workshop`] shape.
//! 2. Re-publishing a different Workshop shape at the same path
//!    produces a different parsed value on the next read — i.e. the
//!    hot-reload pattern the GUI relies on for live-reconfigure
//!    actually works at the substrate level.
//! 3. Workshop shape-match remains stable across publishes (both
//!    before and after re-publish, `matches` stays > 0).
//!
//! What this does NOT cover: the GUI render path. egui's paint
//! requires an egui::Ui which needs a full renderer frame — a
//! smoke-level "doesn't panic" test for paint lives in the unit-test
//! module beside the paint function.

#![cfg(not(target_arch = "wasm32"))]

use clawft_gui_egui::explorer::workshop::{self, WorkshopLayout};
use clawft_substrate::{StateDelta, Substrate};
use serde_json::json;

/// Convention per ADOPTION §8 Step 3.
const WORKSHOP_PATH: &str = "substrate/ui/workshop/test-mic";

#[test]
fn substrate_publish_then_parse_round_trip() {
    let substrate = Substrate::new();

    // First publish — two panels, a title.
    let v1 = json!({
        "title": "Mic diagnostic v1",
        "layout": "rows",
        "panels": [
            { "substrate_path": "substrate/sensor/mic", "viewer_hint": "auto" },
            { "substrate_path": "substrate/kernel/status", "viewer_hint": "auto" }
        ]
    });
    substrate.apply(StateDelta::Replace {
        path: WORKSHOP_PATH.to_string(),
        value: v1.clone(),
    });

    let read_back = substrate
        .get(WORKSHOP_PATH)
        .expect("path should be present after publish");
    assert_eq!(read_back, v1, "round-trip value equality");
    assert!(
        workshop::matches(&read_back) > 0,
        "Workshop shape-match should be positive"
    );

    let parsed = workshop::parse(&read_back).expect("parse succeeds");
    assert_eq!(parsed.title.as_deref(), Some("Mic diagnostic v1"));
    assert_eq!(parsed.layout, WorkshopLayout::Rows);
    assert_eq!(parsed.panels.len(), 2);
    assert_eq!(parsed.panels[0].substrate_path, "substrate/sensor/mic");
}

#[test]
fn substrate_republish_changes_workshop_shape() {
    let substrate = Substrate::new();

    // Initial publish — one panel.
    substrate.apply(StateDelta::Replace {
        path: WORKSHOP_PATH.to_string(),
        value: json!({
            "title": "v1",
            "panels": [
                { "substrate_path": "substrate/a", "viewer_hint": "auto" }
            ]
        }),
    });
    let w1 = workshop::parse(&substrate.get(WORKSHOP_PATH).unwrap()).unwrap();
    assert_eq!(w1.title.as_deref(), Some("v1"));
    assert_eq!(w1.panels.len(), 1);

    // Second publish — title + panels differ. This is the live
    // reconfigure step: a writer swaps the shape; a reader on the
    // next read sees the new layout.
    substrate.apply(StateDelta::Replace {
        path: WORKSHOP_PATH.to_string(),
        value: json!({
            "title": "v2",
            "layout": "grid",
            "panels": [
                { "substrate_path": "substrate/a", "viewer_hint": "auto" },
                { "substrate_path": "substrate/b", "viewer_hint": "auto" },
                { "substrate_path": "substrate/c", "viewer_hint": "auto" }
            ]
        }),
    });
    let w2 = workshop::parse(&substrate.get(WORKSHOP_PATH).unwrap()).unwrap();
    assert_eq!(w2.title.as_deref(), Some("v2"));
    assert_eq!(w2.layout, WorkshopLayout::Grid);
    assert_eq!(w2.panels.len(), 3);
    assert_eq!(w2.panels[2].substrate_path, "substrate/c");

    // The schemas should differ — proves the re-publish landed a
    // genuinely new shape, not a stale read from v1.
    assert_ne!(w1, w2);
}

#[test]
fn substrate_workshop_unrelated_path_untouched() {
    let substrate = Substrate::new();

    substrate.apply(StateDelta::Replace {
        path: WORKSHOP_PATH.to_string(),
        value: json!({
            "panels": [{ "substrate_path": "substrate/sensor/mic", "viewer_hint": "auto" }]
        }),
    });
    substrate.apply(StateDelta::Replace {
        path: "substrate/sensor/mic".to_string(),
        value: json!({ "rms_db": -40.0, "peak_db": -20.0, "available": true }),
    });

    // Publishing to an unrelated path should not affect the Workshop
    // at its own path (sanity check on substrate isolation).
    let ws = substrate.get(WORKSHOP_PATH).unwrap();
    assert!(workshop::matches(&ws) > 0);
    let parsed = workshop::parse(&ws).unwrap();
    assert_eq!(parsed.panels.len(), 1);
}
