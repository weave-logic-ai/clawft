use eframe::egui;

use super::DemoState;

struct Node {
    key: &'static str,
    label: &'static str,
    children: &'static [Node],
}

const TREE: Node = Node {
    key: "root",
    label: "clawft",
    children: &[
        Node {
            key: "kernel",
            label: "kernel",
            children: &[
                Node { key: "kernel/a2a",      label: "a2a",      children: &[] },
                Node { key: "kernel/mesh",     label: "mesh",     children: &[] },
                Node {
                    key: "kernel/services",
                    label: "services",
                    children: &[
                        Node { key: "kernel/services/audio",   label: "audio",   children: &[] },
                        Node { key: "kernel/services/display", label: "display", children: &[] },
                    ],
                },
            ],
        },
        Node {
            key: "weave",
            label: "weave",
            children: &[
                Node { key: "weave/cli",    label: "cli",    children: &[] },
                Node { key: "weave/daemon", label: "daemon", children: &[] },
            ],
        },
    ],
};

pub fn show(ui: &mut egui::Ui, state: &mut DemoState) {
    ui.heading("Tree");
    ui.label("Expandable file-tree mock, click a leaf to select.");
    ui.separator();

    render_node(ui, state, &TREE, 0);
}

fn render_node(ui: &mut egui::Ui, state: &mut DemoState, node: &Node, depth: usize) {
    ui.horizontal(|ui| {
        ui.add_space((depth as f32) * 14.0);
        if node.children.is_empty() {
            ui.label("•");
            ui.label(node.label);
        } else {
            let open = state.tree_open.contains(node.key);
            let arrow = if open { "▾" } else { "▸" };
            if ui.small_button(format!("{arrow} {}", node.label)).clicked() {
                if open {
                    state.tree_open.remove(node.key);
                } else {
                    state.tree_open.insert(node.key);
                }
            }
        }
    });

    if state.tree_open.contains(node.key) {
        for child in node.children {
            render_node(ui, state, child, depth + 1);
        }
    }
}
