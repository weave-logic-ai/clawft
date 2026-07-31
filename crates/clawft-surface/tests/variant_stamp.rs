//! WEFT-431: drive `variant_id` stamping in `CanonResponse` from surface
//! binding (ADR-006 §4 / ADR-007 echo discipline).
//!
//! Acceptance:
//! - Surface bindings can specify `variant_id` (or `variant`).
//! - Static attrs can also author a variant.
//! - `CanonResponse` carries the binding-derived id (not hard-coded 0).
//! - Nodes without a variant still stamp the identity-mapped default 0.

use clawft_surface::builder::{chip, pressable, stack, Surface};
use clawft_surface::parse::parse_surface_toml;
use clawft_surface::render_headless;
use clawft_surface::substrate::OntologySnapshot;
use clawft_surface::tree::{AttrValue, Mode};
use serde_json::json;

#[test]
fn binding_literal_variant_id_stamps_canon_response() {
    let tree = Surface::new("weft-431/bind-literal")
        .modes(&[Mode::Desktop])
        .root(
            stack("/root")
                .child(
                    chip("/root/status")
                        .bind_literal("label", AttrValue::Str("ok".into()))
                        .bind_literal("variant_id", AttrValue::Int(42)),
                )
                .child(
                    pressable("/root/go")
                        .bind_literal("label", AttrValue::Str("Go".into()))
                        .bind_literal("variant", AttrValue::Int(7)),
                ),
        )
        .build();

    let responses = render_headless(&tree, OntologySnapshot::empty());
    assert!(
        !responses.is_empty(),
        "composer must emit at least one CanonResponse"
    );

    let chip = responses
        .iter()
        .find(|r| r.identity.as_ref() == "ui://chip")
        .expect("chip response");
    assert_eq!(chip.variant, 42, "variant_id binding must stamp CanonResponse");

    let press = responses
        .iter()
        .find(|r| r.identity.as_ref() == "ui://pressable")
        .expect("pressable response");
    assert_eq!(press.variant, 7, "variant binding (alias) must stamp CanonResponse");
}

#[test]
fn ontology_binding_drives_variant_selection() {
    // Binding expression `$pulse/variant` resolves against the snapshot
    // so a GEPA pulse can re-stamp the surface without re-authoring.
    let tree = Surface::new("weft-431/bind-path")
        .modes(&[Mode::Desktop])
        .root(
            chip("/root/status")
                .bind("label", "\"healthy\"")
                .bind("variant_id", "$pulse/variant"),
        )
        .build();

    let mut snap = OntologySnapshot::empty();
    snap.put("pulse/variant", json!(99u64));

    let responses = render_headless(&tree, snap);
    let chip = responses
        .iter()
        .find(|r| r.identity.as_ref() == "ui://chip")
        .expect("chip response");
    assert_eq!(
        chip.variant, 99,
        "ontology-bound variant_id must flow into CanonResponse"
    );
}

#[test]
fn attr_variant_id_stamps_when_no_binding() {
    let tree = Surface::new("weft-431/attr")
        .modes(&[Mode::Desktop])
        .root(
            chip("/root/status")
                .bind_literal("label", AttrValue::Str("attr".into()))
                .attr("variant_id", AttrValue::Int(11)),
        )
        .build();

    let responses = render_headless(&tree, OntologySnapshot::empty());
    let chip = responses
        .iter()
        .find(|r| r.identity.as_ref() == "ui://chip")
        .expect("chip response");
    assert_eq!(chip.variant, 11, "attr variant_id is a static authoring path");
}

#[test]
fn missing_variant_defaults_to_identity_mapped_zero() {
    let tree = Surface::new("weft-431/default")
        .modes(&[Mode::Desktop])
        .root(
            chip("/root/status")
                .bind_literal("label", AttrValue::Str("plain".into())),
        )
        .build();

    let responses = render_headless(&tree, OntologySnapshot::empty());
    let chip = responses
        .iter()
        .find(|r| r.identity.as_ref() == "ui://chip")
        .expect("chip response");
    assert_eq!(
        chip.variant, 0,
        "nodes without variant_id keep the stable identity-mapped default"
    );
}

#[test]
fn toml_surface_binding_variant_id() {
    // End-to-end: TOML authoring path → IR → compose → stamped response.
    let toml = r#"
[[surfaces]]
id = "weft-431/toml"
modes = ["desktop"]
title = "Variant stamp"

[surfaces.root]
type = "ui://stack"
id = "/root"

[[surfaces.root.children]]
type = "ui://chip"
id = "/root/pulse"
bindings = { label = "\"v\"", variant_id = "1234" }
"#;
    let tree = parse_surface_toml(toml).expect("parse toml");
    let responses = render_headless(&tree, OntologySnapshot::empty());
    let chip = responses
        .iter()
        .find(|r| r.identity.as_ref() == "ui://chip")
        .expect("chip response");
    assert_eq!(
        chip.variant, 1234,
        "TOML binding variant_id must stamp CanonResponse"
    );
}

#[test]
fn binding_takes_priority_over_attr() {
    let tree = Surface::new("weft-431/priority")
        .modes(&[Mode::Desktop])
        .root(
            chip("/root/status")
                .bind_literal("label", AttrValue::Str("prio".into()))
                .attr("variant_id", AttrValue::Int(1))
                .bind_literal("variant_id", AttrValue::Int(2)),
        )
        .build();

    let responses = render_headless(&tree, OntologySnapshot::empty());
    let chip = responses
        .iter()
        .find(|r| r.identity.as_ref() == "ui://chip")
        .expect("chip response");
    assert_eq!(
        chip.variant, 2,
        "binding must win over static attr when both present"
    );
}
