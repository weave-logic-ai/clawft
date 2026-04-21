//! Composer runtime — walk a [`SurfaceTree`] and drive the canon
//! primitives in this crate. This is the hot path from ADR-016 §4
//! ("Frame time").
//!
//! For M1.5 the wiring covers:
//! - `ui://stack`, `ui://strip`, `ui://grid` containers.
//! - `ui://pressable`, `ui://chip`, `ui://gauge`, `ui://table`,
//!   `ui://stream-view` leaves.
//!
//! Every other canon IRI falls through to [`render_todo`] which
//! paints a visible `"TODO: <iri> not wired in M1.5"` label so the
//! surface still renders without a panic. Sibling M1.6+ milestones
//! light these up one by one.
//!
//! Governance intersection (ADR-006 rule 2) is stubbed: affordances
//! are passed through unfiltered and every node receives
//! `variant_id = 0`. The real wiring lands in M1.6.

use std::cell::RefCell;

use clawft_surface::eval::{eval_binding, Value};
use clawft_surface::substrate::OntologySnapshot;
use clawft_surface::tree::{AffordanceDecl, AttrValue, IdentityIri, SurfaceNode, SurfaceTree};

use crate::canon::{
    pressable::PressableStyle, CanonResponse, CanonWidget, CellSize, Chip, ChipTone, Gauge, Grid,
    Pressable, Stack, StackAxis, StreamView, Strip, StripAxis, Table, TableColumn,
};

/// Main entry point. Walks `tree.root` and drives primitives. Returns
/// a flat list of [`CanonResponse`]s in depth-first order (same order
/// the return-signal walker expects per session-5).
pub fn compose(
    tree: &SurfaceTree,
    snapshot: &OntologySnapshot,
    ui: &mut egui::Ui,
) -> Vec<CanonResponse> {
    let out = RefCell::new(Vec::new());
    render_node(&tree.root, snapshot, ui, &out);
    out.into_inner()
}

fn render_node(
    node: &SurfaceNode,
    snap: &OntologySnapshot,
    ui: &mut egui::Ui,
    out: &RefCell<Vec<CanonResponse>>,
) {
    // Conditional rendering (ADR-016 §6).
    if let Some(when) = &node.when {
        match eval_binding(when, snap) {
            Ok(v) => {
                if !v.as_bool().unwrap_or(false) {
                    return;
                }
            }
            Err(e) => {
                ui.colored_label(
                    egui::Color32::from_rgb(220, 80, 80),
                    format!("when-expr error at {}: {}", node.path, e),
                );
                return;
            }
        }
    }

    match node.kind {
        IdentityIri::Stack => render_stack(node, snap, ui, out),
        IdentityIri::Strip => render_strip(node, snap, ui, out),
        IdentityIri::Grid => render_grid(node, snap, ui, out),
        IdentityIri::Chip => render_chip(node, snap, ui, out),
        IdentityIri::Pressable => render_pressable(node, snap, ui, out),
        IdentityIri::Gauge => render_gauge(node, snap, ui, out),
        IdentityIri::Table => render_table(node, snap, ui, out),
        IdentityIri::StreamView => render_stream_view(node, snap, ui, out),
        other => render_todo(other, &node.path, ui),
    }
}

// ── Containers ─────────────────────────────────────────────────────

fn render_stack(
    node: &SurfaceNode,
    snap: &OntologySnapshot,
    ui: &mut egui::Ui,
    out: &RefCell<Vec<CanonResponse>>,
) {
    let axis = attr_str(node, "axis")
        .and_then(|s| match s {
            "horizontal" => Some(StackAxis::Horizontal),
            "vertical" => Some(StackAxis::Vertical),
            _ => None,
        })
        .unwrap_or(StackAxis::Vertical);
    let wrap = attr_bool(node, "wrap").unwrap_or(false);
    let children = &node.children;

    let stack = Stack::new(&node.path)
        .axis(axis)
        .wrap(wrap)
        .body(|ui: &mut egui::Ui| {
            for child in children {
                render_node(child, snap, ui, out);
            }
        });
    let resp = stack.show(ui);
    out.borrow_mut().push(resp);
}

fn render_strip(
    node: &SurfaceNode,
    snap: &OntologySnapshot,
    ui: &mut egui::Ui,
    out: &RefCell<Vec<CanonResponse>>,
) {
    let axis = attr_str(node, "axis")
        .and_then(|s| match s {
            "horizontal" => Some(StripAxis::Horizontal),
            "vertical" => Some(StripAxis::Vertical),
            _ => None,
        })
        .unwrap_or(StripAxis::Horizontal);

    let cells: Vec<CellSize> = (0..node.children.len()).map(|_| CellSize::Remainder).collect();
    let children = &node.children;

    let strip = Strip::new(&node.path)
        .axis(axis)
        .cells(cells)
        .body(|strip: &mut egui_extras::Strip<'_, '_>| {
            for child in children {
                strip.cell(|ui| {
                    render_node(child, snap, ui, out);
                });
            }
        });
    let resp = strip.show(ui);
    out.borrow_mut().push(resp);
}

fn render_grid(
    node: &SurfaceNode,
    snap: &OntologySnapshot,
    ui: &mut egui::Ui,
    out: &RefCell<Vec<CanonResponse>>,
) {
    let columns = attr_int(node, "columns").unwrap_or(1).max(1) as usize;
    let children = &node.children;

    let grid = Grid::new(&node.path, columns, |ui: &mut egui::Ui| {
        for (i, child) in children.iter().enumerate() {
            render_node(child, snap, ui, out);
            if (i + 1) % columns == 0 {
                ui.end_row();
            }
        }
    });
    let resp = grid.show(ui);
    out.borrow_mut().push(resp);
}

// ── Leaves ─────────────────────────────────────────────────────────

fn render_chip(
    node: &SurfaceNode,
    snap: &OntologySnapshot,
    ui: &mut egui::Ui,
    out: &RefCell<Vec<CanonResponse>>,
) {
    let label = bound_string(node, "label", snap).unwrap_or_else(|| node.path.clone());
    let tone = bound_string(node, "tone", snap)
        .and_then(|s| tone_from_str(&s))
        .unwrap_or(ChipTone::Neutral);

    let mut chip = Chip::new(&node.path, label).tone(tone).variant(0);
    if !node.affordances.is_empty() {
        chip = chip.activatable(true);
    }
    out.borrow_mut().push(chip.show(ui));
}

fn render_pressable(
    node: &SurfaceNode,
    snap: &OntologySnapshot,
    ui: &mut egui::Ui,
    out: &RefCell<Vec<CanonResponse>>,
) {
    let label = bound_string(node, "label", snap).unwrap_or_else(|| node.path.clone());
    let style = attr_str(node, "style")
        .and_then(|s| match s {
            "primary" => Some(PressableStyle::Primary),
            "secondary" => Some(PressableStyle::Secondary),
            "ghost" => Some(PressableStyle::Ghost),
            "destructive" => Some(PressableStyle::Destructive),
            _ => None,
        })
        .unwrap_or(PressableStyle::Primary);
    let enabled = attr_bool(node, "enabled").unwrap_or(true);

    let p = Pressable::new(&node.path, label).style(style).enabled(enabled).variant(0);
    out.borrow_mut().push(p.show(ui));
}

fn render_gauge(
    node: &SurfaceNode,
    snap: &OntologySnapshot,
    ui: &mut egui::Ui,
    out: &RefCell<Vec<CanonResponse>>,
) {
    let value = bound_value(node, "value", snap).and_then(|v| v.as_f64()).unwrap_or(0.0);
    let lo = attr_number(node, "min").unwrap_or(0.0);
    let hi = attr_number(node, "max").unwrap_or(1.0);
    let label = bound_string(node, "label", snap);

    let mut g = Gauge::new(&node.path, value, (lo, hi)).variant(0);
    if let Some(l) = label {
        g = g.label(l);
    }
    out.borrow_mut().push(g.show(ui));
}

fn render_stream_view(
    node: &SurfaceNode,
    snap: &OntologySnapshot,
    ui: &mut egui::Ui,
    out: &RefCell<Vec<CanonResponse>>,
) {
    let lines: Vec<String> = bound_value(node, "stream", snap)
        .and_then(|v| v.as_list())
        .map(|xs| xs.into_iter().map(|v| v.to_display_string()).collect())
        .unwrap_or_default();

    let sv = StreamView::new(&node.path).lines(&lines).variant(0);
    out.borrow_mut().push(sv.show(ui));
}

fn render_table(
    node: &SurfaceNode,
    snap: &OntologySnapshot,
    ui: &mut egui::Ui,
    out: &RefCell<Vec<CanonResponse>>,
) {
    let columns: Vec<TableColumn> = node
        .attrs
        .get("columns")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| a.as_str())
                .map(|s| TableColumn::new(s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let rows: Vec<Value> = bound_value(node, "rows", snap).and_then(|v| v.as_list()).unwrap_or_default();
    let row_count = rows.len();

    let col_keys: Vec<String> = columns.iter().map(|c| c.name.to_string()).collect();

    let t = Table::new(&node.path, &columns)
        .rows(row_count)
        .variant(0)
        .render(|row, idx| {
            if let Some(val) = rows.get(idx) {
                for key in &col_keys {
                    row.col(|ui| {
                        let cell = val.field(key);
                        ui.label(cell.to_display_string());
                    });
                }
            }
        });
    out.borrow_mut().push(t.show(ui));
}

// ── TODO fallback ──────────────────────────────────────────────────

fn render_todo(kind: IdentityIri, path: &str, ui: &mut egui::Ui) {
    ui.label(
        egui::RichText::new(format!(
            "TODO: {} not wired in M1.5 ({})",
            kind.as_iri(),
            path
        ))
        .color(egui::Color32::from_rgb(200, 160, 60))
        .italics(),
    );
}

// ── Binding / attribute helpers ────────────────────────────────────

fn bound_value(node: &SurfaceNode, slot: &str, snap: &OntologySnapshot) -> Option<Value> {
    node.bindings.get(slot).and_then(|b| eval_binding(b, snap).ok())
}

fn bound_string(node: &SurfaceNode, slot: &str, snap: &OntologySnapshot) -> Option<String> {
    let v = bound_value(node, slot, snap)?;
    Some(v.to_display_string())
}

fn tone_from_str(s: &str) -> Option<ChipTone> {
    Some(match s {
        "ok" | "healthy" => ChipTone::Ok,
        "warn" | "at_risk" => ChipTone::Warn,
        "crit" | "down" | "error" => ChipTone::Crit,
        "info" => ChipTone::Info,
        "neutral" => ChipTone::Neutral,
        _ => return None,
    })
}

fn attr_str<'a>(node: &'a SurfaceNode, key: &str) -> Option<&'a str> {
    node.attrs.get(key).and_then(AttrValue::as_str)
}

fn attr_bool(node: &SurfaceNode, key: &str) -> Option<bool> {
    node.attrs.get(key).and_then(AttrValue::as_bool)
}

fn attr_int(node: &SurfaceNode, key: &str) -> Option<i64> {
    node.attrs.get(key).and_then(AttrValue::as_int)
}

fn attr_number(node: &SurfaceNode, key: &str) -> Option<f64> {
    node.attrs.get(key).and_then(AttrValue::as_number)
}

/// Exposed so a future governance pass can intersect affordances
/// against a policy. Currently identity — ADR-006 rule 2 TODO.
pub fn honest_affordances(_node: &SurfaceNode, raw: &[AffordanceDecl]) -> Vec<AffordanceDecl> {
    raw.to_vec()
}
