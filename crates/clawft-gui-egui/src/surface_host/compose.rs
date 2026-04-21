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
//! **M1.5.1a**: affordances declared on a `ui://table` or `ui://gauge`
//! are wired end-to-end. A row click on a table with an affordance, or
//! a click on a gauge's affordance button, produces a
//! [`PendingDispatch`] in the returned [`ComposeOutcome`] which the
//! desktop shell drains and submits via the `Live` RPC bridge. For a
//! table row, `params.pid` is pulled from the clicked row's `pid`
//! field; for a gauge, `params.name` is the last path segment of the
//! node id. The fixture's `rpc.` verb prefix is stripped before
//! dispatch so `rpc.kernel.kill-process` reaches the daemon as
//! `kernel.kill-process` (matching the extension's ALLOWED_METHODS
//! allowlist + the daemon handlers added in M1.5.1a).
//!
//! Governance intersection (ADR-006 rule 2) is still stubbed: every
//! node receives `variant_id = 0` and affordances are passed through
//! unfiltered. The honest GEPA-gated intersection lands with M2's
//! active-radar loop — at that point [`honest_affordances`] grows a
//! real implementation and the composer stops dispatching verbs the
//! gate would have denied.

use std::cell::RefCell;

use clawft_surface::eval::{eval_binding, Value};
use clawft_surface::substrate::OntologySnapshot;
use clawft_surface::tree::{AffordanceDecl, AttrValue, IdentityIri, SurfaceNode, SurfaceTree};

use crate::canon::{
    pressable::PressableStyle, CanonResponse, CanonWidget, CellSize, Chip, ChipTone, Gauge, Grid,
    Pressable, Stack, StackAxis, StreamView, Strip, StripAxis, Table, TableColumn,
};

/// A verb activation picked up by the composer during a frame. The
/// caller (the desktop shell's admin-app render path) drains these
/// after `compose()` returns and submits them through the `Live` RPC
/// bridge.
///
/// The shape is deliberately flat + serde-friendly so the desktop
/// doesn't have to know which primitive produced the dispatch — just
/// the verb string and the param object.
#[derive(Debug, Clone)]
pub struct PendingDispatch {
    /// Surface-tree path of the node whose affordance fired (e.g.
    /// `/root/processes`).
    pub source_path: String,
    /// Declared affordance name (e.g. `kill`, `restart`).
    pub affordance: String,
    /// RPC method already normalized — the `rpc.` prefix has been
    /// stripped so this maps directly to a daemon handler name.
    pub verb: String,
    /// JSON params for the RPC call. Shape is verb-specific
    /// (`{"pid": <u64>}` for `kernel.kill-process`,
    /// `{"name": <str>}` for `kernel.restart-service`).
    pub params: serde_json::Value,
}

/// Per-frame output from [`compose`]. The composer collects both the
/// flat list of [`CanonResponse`]s (for the observation walker) and
/// the list of affordance dispatches (for the RPC bridge). Both are
/// in depth-first surface-tree order.
#[derive(Debug, Default)]
pub struct ComposeOutcome {
    pub responses: Vec<CanonResponse>,
    pub dispatches: Vec<PendingDispatch>,
}

/// Internal call frame — every recursive `render_*` takes these
/// together so affordance-emitting primitives can push dispatches
/// alongside their CanonResponse without plumbing a second argument
/// through every signature.
struct Frame<'a> {
    responses: &'a RefCell<Vec<CanonResponse>>,
    dispatches: &'a RefCell<Vec<PendingDispatch>>,
}

/// Main entry point. Walks `tree.root` and drives primitives. Returns
/// a [`ComposeOutcome`] with the flat response list + any pending
/// RPC dispatches produced by affordance activations this frame.
pub fn compose(
    tree: &SurfaceTree,
    snapshot: &OntologySnapshot,
    ui: &mut egui::Ui,
) -> ComposeOutcome {
    let responses = RefCell::new(Vec::new());
    let dispatches = RefCell::new(Vec::new());
    let frame = Frame {
        responses: &responses,
        dispatches: &dispatches,
    };
    render_node(&tree.root, snapshot, ui, &frame);
    ComposeOutcome {
        responses: responses.into_inner(),
        dispatches: dispatches.into_inner(),
    }
}

fn render_node(
    node: &SurfaceNode,
    snap: &OntologySnapshot,
    ui: &mut egui::Ui,
    frame: &Frame<'_>,
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
        IdentityIri::Stack => render_stack(node, snap, ui, frame),
        IdentityIri::Strip => render_strip(node, snap, ui, frame),
        IdentityIri::Grid => render_grid(node, snap, ui, frame),
        IdentityIri::Chip => render_chip(node, snap, ui, frame),
        IdentityIri::Pressable => render_pressable(node, snap, ui, frame),
        IdentityIri::Gauge => render_gauge(node, snap, ui, frame),
        IdentityIri::Table => render_table(node, snap, ui, frame),
        IdentityIri::StreamView => render_stream_view(node, snap, ui, frame),
        other => render_todo(other, &node.path, ui),
    }
}

// ── Containers ─────────────────────────────────────────────────────

fn render_stack(
    node: &SurfaceNode,
    snap: &OntologySnapshot,
    ui: &mut egui::Ui,
    frame: &Frame<'_>,
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
                render_node(child, snap, ui, frame);
            }
        });
    let resp = stack.show(ui);
    frame.responses.borrow_mut().push(resp);
}

fn render_strip(
    node: &SurfaceNode,
    snap: &OntologySnapshot,
    ui: &mut egui::Ui,
    frame: &Frame<'_>,
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
                    render_node(child, snap, ui, frame);
                });
            }
        });
    let resp = strip.show(ui);
    frame.responses.borrow_mut().push(resp);
}

fn render_grid(
    node: &SurfaceNode,
    snap: &OntologySnapshot,
    ui: &mut egui::Ui,
    frame: &Frame<'_>,
) {
    let columns = attr_int(node, "columns").unwrap_or(1).max(1) as usize;
    let children = &node.children;

    // Wrap each child in a framed card so the grid reads as
    // partitioned quadrants rather than a stream of orphan widgets.
    // M1.5.1a polish — the fixture is a 2×2 layout that was visually
    // collapsing in narrow webviews.
    let grid = Grid::new(&node.path, columns, |ui: &mut egui::Ui| {
        for (i, child) in children.iter().enumerate() {
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::same(8.0))
                .show(ui, |ui| {
                    render_node(child, snap, ui, frame);
                });
            if (i + 1) % columns == 0 {
                ui.end_row();
            }
        }
    });
    let resp = grid.show(ui);
    frame.responses.borrow_mut().push(resp);
}

// ── Leaves ─────────────────────────────────────────────────────────

fn render_chip(
    node: &SurfaceNode,
    snap: &OntologySnapshot,
    ui: &mut egui::Ui,
    frame: &Frame<'_>,
) {
    let label = bound_string(node, "label", snap).unwrap_or_else(|| node.path.clone());
    let tone = bound_string(node, "tone", snap)
        .and_then(|s| tone_from_str(&s))
        .unwrap_or(ChipTone::Neutral);

    let mut chip = Chip::new(&node.path, label).tone(tone).variant(0);
    if !node.affordances.is_empty() {
        chip = chip.activatable(true);
    }
    let resp = chip.show(ui);
    // If the chip was activated this frame and the node declares a
    // non-built-in affordance (e.g. `activate` mapped to a real RPC
    // verb), dispatch it. The admin fixture doesn't declare chip
    // affordances today, but this keeps the path alive for any future
    // surface that binds a chip to a verb.
    if resp.inner.clicked()
        && let Some(dispatch) = build_dispatch(node, None)
    {
        frame.dispatches.borrow_mut().push(dispatch);
    }
    frame.responses.borrow_mut().push(resp);
}

fn render_pressable(
    node: &SurfaceNode,
    snap: &OntologySnapshot,
    ui: &mut egui::Ui,
    frame: &Frame<'_>,
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
    let resp = p.show(ui);
    if resp.inner.clicked()
        && let Some(dispatch) = build_dispatch(node, None)
    {
        frame.dispatches.borrow_mut().push(dispatch);
    }
    frame.responses.borrow_mut().push(resp);
}

fn render_gauge(
    node: &SurfaceNode,
    snap: &OntologySnapshot,
    ui: &mut egui::Ui,
    frame: &Frame<'_>,
) {
    let value = bound_value(node, "value", snap).and_then(|v| v.as_f64()).unwrap_or(0.0);
    let lo = attr_number(node, "min").unwrap_or(0.0);
    let hi = attr_number(node, "max").unwrap_or(1.0);
    let label = bound_string(node, "label", snap);

    let mut g = Gauge::new(&node.path, value, (lo, hi)).variant(0);
    if let Some(l) = label {
        g = g.label(l);
    }
    let resp = g.show(ui);
    frame.responses.borrow_mut().push(resp);

    // If the node declares affordances, render a small action strip
    // underneath the gauge. Gauges themselves are read-only (the canon
    // `ui://gauge` has an empty AFFORDANCES list) so the composer adds
    // explicit action buttons per affordance declaration — this reads
    // better than pretending a progress bar is clickable.
    if !node.affordances.is_empty() {
        ui.horizontal(|ui| {
            for aff in &node.affordances {
                // Pretty label: use the declared affordance name
                // capitalised. `kill` → `Kill`, `restart` → `Restart`.
                let label = prettify(&aff.name);
                if ui
                    .small_button(format!("↻ {label}"))
                    .on_hover_text(format!("{} — {}", aff.name, aff.verb))
                    .clicked()
                    && let Some(dispatch) = build_dispatch(node, Some(aff))
                {
                    frame.dispatches.borrow_mut().push(dispatch);
                }
            }
        });
    }
}

fn render_stream_view(
    node: &SurfaceNode,
    snap: &OntologySnapshot,
    ui: &mut egui::Ui,
    frame: &Frame<'_>,
) {
    let lines: Vec<String> = bound_value(node, "stream", snap)
        .and_then(|v| v.as_list())
        .map(|xs| xs.into_iter().map(|v| v.to_display_string()).collect())
        .unwrap_or_default();

    // Clamp the stream-view to the current ui's available width so a
    // long log line can't push the parent container off-screen. The
    // underlying `StreamView` widget doesn't impose a width cap.
    let width = ui.available_width();
    let sv = StreamView::new(&node.path)
        .lines(&lines)
        .desired_width(width)
        .variant(0);
    frame.responses.borrow_mut().push(sv.show(ui));
}

fn render_table(
    node: &SurfaceNode,
    snap: &OntologySnapshot,
    ui: &mut egui::Ui,
    frame: &Frame<'_>,
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

    // Row-click state is collected via the first cell of each row as a
    // selectable label. The primary cell is whichever column contains
    // the primary key (`pid` for kernel.ps, typically first column).
    // We capture the click index inside the render closure via a
    // RefCell so the row dispatch logic can read it after show.
    let clicked_row: RefCell<Option<usize>> = RefCell::new(None);

    let t = Table::new(&node.path, &columns)
        .rows(row_count)
        .variant(0)
        .render(|row, idx| {
            if let Some(val) = rows.get(idx) {
                for (i, key) in col_keys.iter().enumerate() {
                    row.col(|ui| {
                        let cell = val.field(key);
                        let text = cell.to_display_string();
                        if i == 0 && !node.affordances.is_empty() {
                            // First column is the click target when
                            // the table declares row-level affordances.
                            if ui.selectable_label(false, text).clicked() {
                                *clicked_row.borrow_mut() = Some(idx);
                            }
                        } else {
                            ui.label(text);
                        }
                    });
                }
            }
        });
    let (resp, _outcome) = t.show_with_outcome(ui);
    frame.responses.borrow_mut().push(resp);

    // If a row was clicked and the node has a row-level affordance,
    // extract the row's `pid` (or the first column's value if there's
    // no `pid` field) and dispatch. The params shape is per-verb — for
    // `kernel.kill-process` the daemon expects `{"pid": u64}`.
    if let Some(idx) = *clicked_row.borrow()
        && let Some(aff) = node.affordances.first()
        && let Some(row_val) = rows.get(idx)
    {
        let params = row_params_for(&aff.verb, row_val);
        let verb = strip_rpc_prefix(&aff.verb);
        frame.dispatches.borrow_mut().push(PendingDispatch {
            source_path: node.path.clone(),
            affordance: aff.name.clone(),
            verb,
            params,
        });
    }
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

// ── Affordance dispatch helpers ─────────────────────────────────────

/// Strip the WSP namespace prefix so a fixture verb
/// `rpc.kernel.kill-process` dispatches as the daemon handler
/// `kernel.kill-process`. The daemon / extension allowlist use the
/// bare form.
fn strip_rpc_prefix(verb: &str) -> String {
    verb.strip_prefix("rpc.")
        .map(|s| s.to_string())
        .unwrap_or_else(|| verb.to_string())
}

/// Build the JSON params object for a verb that acts on a single
/// table row. Verb-specific extraction — for the M1.5.1a verbs:
/// `kernel.kill-process` pulls `pid` from the row.
fn row_params_for(verb: &str, row: &Value) -> serde_json::Value {
    let bare = strip_rpc_prefix(verb);
    match bare.as_str() {
        "kernel.kill-process" => {
            let pid = row.field("pid").as_i64().unwrap_or(0);
            serde_json::json!({ "pid": pid })
        }
        _ => serde_json::json!({}),
    }
}

/// Build a dispatch for a node whose primary affordance fires at the
/// node level (gauge button, chip activate, pressable click). Returns
/// `None` if the node has no affordances. For a specific affordance,
/// pass `Some(&aff)`; otherwise the first declared affordance is used.
fn build_dispatch(node: &SurfaceNode, aff: Option<&AffordanceDecl>) -> Option<PendingDispatch> {
    let aff = aff.or_else(|| node.affordances.first())?;
    let verb = strip_rpc_prefix(&aff.verb);
    let params = node_params_for(&aff.verb, node);
    Some(PendingDispatch {
        source_path: node.path.clone(),
        affordance: aff.name.clone(),
        verb,
        params,
    })
}

/// Params for a node-level affordance. For `kernel.restart-service`
/// the name comes from the last segment of the node path
/// (e.g. `/root/services/mesh-listener` → `"mesh-listener"`).
fn node_params_for(verb: &str, node: &SurfaceNode) -> serde_json::Value {
    let bare = strip_rpc_prefix(verb);
    match bare.as_str() {
        "kernel.restart-service" => {
            let name = node.path.rsplit('/').next().unwrap_or(&node.path);
            serde_json::json!({ "name": name })
        }
        _ => serde_json::json!({}),
    }
}

fn prettify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut first = true;
    for c in s.chars() {
        if first {
            out.extend(c.to_uppercase());
            first = false;
        } else {
            out.push(c);
        }
    }
    out
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
