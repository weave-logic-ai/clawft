//! Ported block demos (mock data, no kernel IPC yet).

pub mod budget;
pub mod button;
pub mod code;
pub mod layout;
pub mod oscilloscope;
pub mod overview;
pub mod status;
pub mod table;
pub mod tabs;
pub mod terminal;
pub mod text;
pub mod tree;

/// Demo-wide state so interactive blocks (counter, tabs selection, table
/// sorting, terminal history, oscilloscope time) persist across frames.
pub struct DemoState {
    pub counter: u32,
    pub tab_idx: usize,
    pub table_sort_col: Option<usize>,
    pub table_sort_asc: bool,
    pub selected_row: Option<usize>,
    pub tree_open: std::collections::HashSet<&'static str>,
    pub terminal_input: String,
    pub terminal_history: Vec<(TerminalLineKind, String)>,
    pub scope_t: f32,
    pub scope_samples: std::collections::VecDeque<(f64, f64)>,
}

#[derive(Copy, Clone)]
pub enum TerminalLineKind {
    Input,
    Output,
    Error,
}

impl Default for DemoState {
    fn default() -> Self {
        let mut tree_open = std::collections::HashSet::new();
        tree_open.insert("kernel");
        tree_open.insert("kernel/services");

        Self {
            counter: 0,
            tab_idx: 0,
            table_sort_col: None,
            table_sort_asc: true,
            selected_row: None,
            tree_open,
            terminal_input: String::new(),
            terminal_history: vec![
                (
                    TerminalLineKind::Output,
                    "weft v0.6.17 (egui spike demo)".into(),
                ),
                (
                    TerminalLineKind::Output,
                    "Type `help` for mock commands.".into(),
                ),
            ],
            scope_t: 0.0,
            scope_samples: std::collections::VecDeque::with_capacity(512),
        }
    }
}
