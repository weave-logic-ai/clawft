use eframe::egui;
use egui_extras::{Column, TableBuilder};

use super::DemoState;

struct Session {
    id: &'static str,
    msgs: u32,
    cost: f32,
    updated: &'static str,
}

const ROWS: &[Session] = &[
    Session { id: "sess-01", msgs: 142, cost: 0.84, updated: "3m ago" },
    Session { id: "sess-02", msgs:  17, cost: 0.09, updated: "12m ago" },
    Session { id: "sess-03", msgs: 603, cost: 3.12, updated: "1h ago" },
    Session { id: "sess-04", msgs:   4, cost: 0.02, updated: "2d ago" },
    Session { id: "sess-05", msgs:  88, cost: 0.44, updated: "just now" },
];

pub fn show(ui: &mut egui::Ui, state: &mut DemoState) {
    ui.heading("Table");
    ui.label("Sortable sessions table via egui_extras::TableBuilder.");
    ui.separator();

    let mut indices: Vec<usize> = (0..ROWS.len()).collect();
    if let Some(col) = state.table_sort_col {
        indices.sort_by(|&a, &b| {
            let (ra, rb) = (&ROWS[a], &ROWS[b]);
            let ord = match col {
                0 => ra.id.cmp(rb.id),
                1 => ra.msgs.cmp(&rb.msgs),
                2 => ra.cost.partial_cmp(&rb.cost).unwrap_or(std::cmp::Ordering::Equal),
                _ => ra.updated.cmp(rb.updated),
            };
            if state.table_sort_asc { ord } else { ord.reverse() }
        });
    }

    TableBuilder::new(ui)
        .striped(true)
        .column(Column::auto().at_least(110.0))
        .column(Column::auto().at_least(80.0))
        .column(Column::auto().at_least(80.0))
        .column(Column::remainder())
        .header(24.0, |mut h| {
            for (i, label) in ["Session", "Msgs", "Cost $", "Updated"].iter().enumerate() {
                h.col(|ui| {
                    if ui.strong(*label).clicked() {
                        toggle_sort(state, i);
                    }
                });
            }
        })
        .body(|mut body| {
            for &i in &indices {
                let row = &ROWS[i];
                let selected = state.selected_row == Some(i);
                body.row(22.0, |mut r| {
                    r.col(|ui| {
                        if ui.selectable_label(selected, row.id).clicked() {
                            state.selected_row = Some(i);
                        }
                    });
                    r.col(|ui| { ui.monospace(row.msgs.to_string()); });
                    r.col(|ui| { ui.monospace(format!("{:.2}", row.cost)); });
                    r.col(|ui| { ui.label(row.updated); });
                });
            }
        });
}

fn toggle_sort(state: &mut DemoState, col: usize) {
    if state.table_sort_col == Some(col) {
        state.table_sort_asc = !state.table_sort_asc;
    } else {
        state.table_sort_col = Some(col);
        state.table_sort_asc = true;
    }
}
