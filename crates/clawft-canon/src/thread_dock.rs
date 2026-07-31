//! `ui://thread-dock` — column-per-agent parallel output (WEFT-284).
//!
//! Compositional extension of Dock for multi-agent swarm UX
//! (session-7 §Multi-agent parallel output, ROADMAP M2 ThreadDock).
//! Multiple parallel agent streams never interleave in a single
//! scroll: each agent owns a monotonic column; one **focus lane**
//! receives keyboard / composer attention; non-focused columns show
//! presence pips (phase + token rate).
//!
//! Not an ADR-001 row amendment — the frozen 21-item table stays
//! intact. ThreadDock is an app-level composition over
//! StreamView-style line tails, exposed as a first-class
//! `CanonWidget` so the VSCode panel and chat host share one type.
//!
//! Governance: agent columns never raise modals (session-7). Anything
//! that would be modal becomes a tray chip outside this primitive.

use std::borrow::Cow;

use egui;

use super::CanonWidget;
use super::response::CanonResponse;
use super::stream_view::StreamView;
use super::types::{Affordance, Confidence, IdentityUri, MutationAxis, Tooltip, VariantId};

const IDENTITY: &str = "ui://thread-dock";

static AFFORDANCES: &[Affordance] = &[
    Affordance {
        name: Cow::Borrowed("switch-focus"),
        verb: Cow::Borrowed("wsp.set"),
        actors: &[],
        args_schema: None,
        reorderable: false,
    },
    Affordance {
        name: Cow::Borrowed("interrupt"),
        verb: Cow::Borrowed("wsp.invoke"),
        actors: &[],
        args_schema: None,
        reorderable: false,
    },
    Affordance {
        name: Cow::Borrowed("close-thread"),
        verb: Cow::Borrowed("wsp.invoke"),
        actors: &[],
        args_schema: None,
        reorderable: false,
    },
];

static MUTATION_AXES: &[MutationAxis] = &[
    MutationAxis::new("column-min-width"),
    MutationAxis::new("presence-pip-style"),
    MutationAxis::new("focus-lane-chrome"),
];

/// Live phase for an agent column's presence pip.
///
/// Maps to session-7 states: thinking / streaming / awaiting-input /
/// done (+ idle start and error terminal).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum ThreadPhase {
    #[default]
    Idle,
    Thinking,
    Streaming,
    AwaitingInput,
    Done,
    Error,
}

impl ThreadPhase {
    /// Stable wire / UI label (lowercase kebab).
    pub fn as_str(self) -> &'static str {
        match self {
            ThreadPhase::Idle => "idle",
            ThreadPhase::Thinking => "thinking",
            ThreadPhase::Streaming => "streaming",
            ThreadPhase::AwaitingInput => "awaiting-input",
            ThreadPhase::Done => "done",
            ThreadPhase::Error => "error",
        }
    }

    /// Parse a wire label; unknown → `None`.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "idle" => Some(ThreadPhase::Idle),
            "thinking" => Some(ThreadPhase::Thinking),
            "streaming" | "generating" => Some(ThreadPhase::Streaming),
            "awaiting-input" | "awaiting" => Some(ThreadPhase::AwaitingInput),
            "done" => Some(ThreadPhase::Done),
            "error" => Some(ThreadPhase::Error),
            _ => None,
        }
    }

    /// True while the agent is actively producing output.
    pub fn is_active(self) -> bool {
        matches!(
            self,
            ThreadPhase::Thinking | ThreadPhase::Streaming | ThreadPhase::AwaitingInput
        )
    }

    fn pip_color(self) -> egui::Color32 {
        match self {
            ThreadPhase::Idle => egui::Color32::from_gray(90),
            ThreadPhase::Thinking => egui::Color32::from_rgb(120, 180, 255),
            ThreadPhase::Streaming => egui::Color32::from_rgb(110, 210, 140),
            ThreadPhase::AwaitingInput => egui::Color32::from_rgb(255, 205, 90),
            ThreadPhase::Done => egui::Color32::from_rgb(90, 160, 120),
            ThreadPhase::Error => egui::Color32::from_rgb(255, 120, 120),
        }
    }
}

/// One parallel agent column — ontology-addressable, streamable.
///
/// Ordering of [`Self::lines`] is monotonic per agent; cross-thread
/// interleaving is structurally impossible because each thread owns
/// its buffer.
#[derive(Clone, Debug)]
pub struct AgentThread {
    /// Stable thread id (substrate path segment or swarm agent id).
    pub id: String,
    /// Human / voice label ("coder", "planner", "supervisor").
    pub label: String,
    /// Presence phase for the header pip.
    pub phase: ThreadPhase,
    /// Monotonic output lines (or message chunks).
    pub lines: Vec<String>,
    /// Approximate tokens/sec for the presence pip (0 = unknown).
    pub token_rate: f32,
    /// Active-radar variant-id for the last pulse that fed this column.
    pub variant_id: VariantId,
}

impl AgentThread {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            phase: ThreadPhase::Idle,
            lines: Vec::new(),
            token_rate: 0.0,
            variant_id: 0,
        }
    }

    pub fn phase(mut self, phase: ThreadPhase) -> Self {
        self.phase = phase;
        self
    }

    pub fn lines(mut self, lines: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.lines = lines.into_iter().map(Into::into).collect();
        self
    }

    pub fn token_rate(mut self, rate: f32) -> Self {
        self.token_rate = rate.max(0.0);
        self
    }

    pub fn variant(mut self, variant: VariantId) -> Self {
        self.variant_id = variant;
        self
    }

    /// Append one output line (never interleaves with other threads).
    pub fn append(&mut self, line: impl Into<String>) {
        self.lines.push(line.into());
    }

    /// Append a chunk that may contain newlines; splits into lines.
    pub fn append_chunk(&mut self, chunk: &str) {
        if chunk.is_empty() {
            return;
        }
        for line in chunk.split('\n') {
            // Keep empty lines from `\n\n` so markdown spacing survives.
            self.lines.push(line.to_owned());
        }
        // If the chunk did not end with `\n` and we already had a partial
        // last line, the split above already placed the tail as its own
        // entry — good enough for stream tails (callers that need true
        // append-to-last-line can set lines themselves).
    }
}

/// Caller-owned dock state — persists geometry of focus across frames.
#[derive(Clone, Debug, Default)]
pub struct ThreadDockState {
    threads: Vec<AgentThread>,
    /// Index of the focus lane (keyboard / composer target).
    focused: usize,
}

impl ThreadDockState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed with ≥2 agent columns (typical swarm / supervisor pair).
    pub fn with_threads(threads: impl IntoIterator<Item = AgentThread>) -> Self {
        let threads: Vec<_> = threads.into_iter().collect();
        Self {
            focused: 0,
            threads,
        }
    }

    pub fn len(&self) -> usize {
        self.threads.len()
    }

    pub fn is_empty(&self) -> bool {
        self.threads.is_empty()
    }

    /// True when the dock shows parallel columns (≥2 streams).
    pub fn is_parallel(&self) -> bool {
        self.threads.len() >= 2
    }

    pub fn threads(&self) -> &[AgentThread] {
        &self.threads
    }

    pub fn threads_mut(&mut self) -> &mut [AgentThread] {
        &mut self.threads
    }

    pub fn focused_index(&self) -> usize {
        self.focused
    }

    pub fn focused(&self) -> Option<&AgentThread> {
        self.threads.get(self.focused)
    }

    pub fn focused_mut(&mut self) -> Option<&mut AgentThread> {
        self.threads.get_mut(self.focused)
    }

    /// Push a thread; returns its index. Focus stays put unless this
    /// is the first thread.
    pub fn push(&mut self, thread: AgentThread) -> usize {
        self.threads.push(thread);
        let idx = self.threads.len() - 1;
        if self.threads.len() == 1 {
            self.focused = 0;
        }
        idx
    }

    /// Select focus lane by index. No-op on OOB.
    pub fn focus(&mut self, idx: usize) {
        if idx < self.threads.len() {
            self.focused = idx;
        }
    }

    /// Select focus lane by thread id. Returns whether found.
    pub fn focus_by_id(&mut self, id: &str) -> bool {
        if let Some(idx) = self.threads.iter().position(|t| t.id == id) {
            self.focused = idx;
            true
        } else {
            false
        }
    }

    pub fn get(&self, id: &str) -> Option<&AgentThread> {
        self.threads.iter().find(|t| t.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut AgentThread> {
        self.threads.iter_mut().find(|t| t.id == id)
    }

    /// Append a line to a thread by id. Returns false if missing.
    pub fn append_line(&mut self, id: &str, line: impl Into<String>) -> bool {
        match self.get_mut(id) {
            Some(t) => {
                t.append(line);
                true
            }
            None => false,
        }
    }

    /// Replace / grow a thread's line buffer from an accumulated stream
    /// draft (WEFT-253 style: full text, not delta). Splits on newlines.
    pub fn set_stream_text(&mut self, id: &str, text: &str) -> bool {
        match self.get_mut(id) {
            Some(t) => {
                t.lines = if text.is_empty() {
                    Vec::new()
                } else {
                    text.split('\n').map(str::to_owned).collect()
                };
                true
            }
            None => false,
        }
    }

    pub fn set_phase(&mut self, id: &str, phase: ThreadPhase) -> bool {
        match self.get_mut(id) {
            Some(t) => {
                t.phase = phase;
                true
            }
            None => false,
        }
    }

    pub fn set_token_rate(&mut self, id: &str, rate: f32) -> bool {
        match self.get_mut(id) {
            Some(t) => {
                t.token_rate = rate.max(0.0);
                true
            }
            None => false,
        }
    }

    /// Remove a thread by id. Adjusts focus if needed. Returns whether
    /// a thread was removed.
    pub fn remove(&mut self, id: &str) -> bool {
        let Some(idx) = self.threads.iter().position(|t| t.id == id) else {
            return false;
        };
        self.threads.remove(idx);
        if self.threads.is_empty() {
            self.focused = 0;
        } else if self.focused > idx {
            self.focused -= 1;
        } else if self.focused >= self.threads.len() {
            self.focused = self.threads.len() - 1;
        }
        true
    }

    /// Clear all threads and reset focus.
    pub fn clear(&mut self) {
        self.threads.clear();
        self.focused = 0;
    }
}

/// Column-per-agent parallel output dock.
///
/// Callers own [`ThreadDockState`] across frames. Body of each column
/// is a live-tailing [`StreamView`] of that thread's lines.
pub struct ThreadDock<'a> {
    id_source: egui::Id,
    state: &'a mut ThreadDockState,
    min_height: f32,
    max_height: Option<f32>,
    column_min_width: f32,
    show_close: bool,
    tooltip: Option<Tooltip>,
    variant: VariantId,
}

impl<'a> ThreadDock<'a> {
    pub fn new(id_source: impl std::hash::Hash, state: &'a mut ThreadDockState) -> Self {
        Self {
            id_source: egui::Id::new(("canon.thread_dock", id_source)),
            state,
            min_height: 160.0,
            max_height: None,
            column_min_width: 140.0,
            show_close: false,
            tooltip: None,
            variant: 0,
        }
    }

    pub fn min_height(mut self, h: f32) -> Self {
        self.min_height = h;
        self
    }

    pub fn max_height(mut self, h: f32) -> Self {
        self.max_height = Some(h);
        self
    }

    pub fn column_min_width(mut self, w: f32) -> Self {
        self.column_min_width = w.max(80.0);
        self
    }

    /// Show a close affordance on each column header.
    pub fn show_close(mut self, show: bool) -> Self {
        self.show_close = show;
        self
    }

    pub fn tooltip(mut self, text: impl Into<Tooltip>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    pub fn variant(mut self, variant: VariantId) -> Self {
        self.variant = variant;
        self
    }
}

impl CanonWidget for ThreadDock<'_> {
    fn id(&self) -> egui::Id {
        self.id_source
    }

    fn identity_uri(&self) -> IdentityUri {
        Cow::Borrowed(IDENTITY)
    }

    fn affordances(&self) -> &[Affordance] {
        AFFORDANCES
    }

    fn confidence(&self) -> Confidence {
        Confidence::deterministic()
    }

    fn variant_id(&self) -> VariantId {
        self.variant
    }

    fn mutation_axes(&self) -> &[MutationAxis] {
        MUTATION_AXES
    }

    fn tooltip(&self) -> Option<&Tooltip> {
        self.tooltip.as_ref()
    }

    fn show(self, ui: &mut egui::Ui) -> CanonResponse {
        let id = self.id_source;
        let variant = self.variant;
        let tooltip = self.tooltip.clone();
        let min_height = self.min_height;
        let max_height = self.max_height;
        let column_min_width = self.column_min_width;
        let show_close = self.show_close;
        let state = self.state;

        let mut chosen: Option<&'static str> = None;
        let mut close_id: Option<String> = None;

        let scope = ui.scope(|ui| {
            ui.set_min_height(min_height);
            if let Some(h) = max_height {
                ui.set_max_height(h);
            }

            if state.threads.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        egui::RichText::new("No agent threads")
                            .italics()
                            .color(egui::Color32::from_gray(140)),
                    );
                });
                return;
            }

            let n = state.threads.len();
            let avail = ui.available_width();
            let col_w = (avail / n as f32).max(column_min_width);

            ui.horizontal_top(|ui| {
                for idx in 0..n {
                    let focused = state.focused == idx;
                    // Borrow one thread at a time for paint; mutate focus after.
                    let thread_id = state.threads[idx].id.clone();
                    let label = state.threads[idx].label.clone();
                    let phase = state.threads[idx].phase;
                    let token_rate = state.threads[idx].token_rate;
                    let lines = state.threads[idx].lines.clone();

                    let frame = egui::Frame::new()
                        .fill(if focused {
                            egui::Color32::from_rgb(16, 22, 32)
                        } else {
                            egui::Color32::from_rgb(10, 12, 16)
                        })
                        .stroke(egui::Stroke::new(
                            if focused { 1.5 } else { 1.0 },
                            if focused {
                                egui::Color32::from_rgb(80, 140, 220)
                            } else {
                                egui::Color32::from_gray(40)
                            },
                        ))
                        .corner_radius(4.0)
                        .inner_margin(egui::Margin::symmetric(6, 6));

                    let col_resp = frame
                        .show(ui, |ui| {
                            ui.set_min_width(col_w - 8.0);
                            ui.set_max_width(col_w - 4.0);
                            ui.set_min_height(min_height - 8.0);

                            // Header: pip + label + rate + optional close.
                            ui.horizontal(|ui| {
                                let (pip_rect, _) = ui.allocate_exact_size(
                                    egui::vec2(8.0, 8.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter()
                                    .circle_filled(pip_rect.center(), 3.5, phase.pip_color());

                                let title = if focused {
                                    egui::RichText::new(&label).strong()
                                } else {
                                    egui::RichText::new(&label)
                                };
                                if ui
                                    .add(egui::Label::new(title).sense(egui::Sense::click()))
                                    .clicked()
                                    && !focused
                                {
                                    chosen = Some("switch-focus");
                                    state.focus(idx);
                                }

                                ui.label(
                                    egui::RichText::new(phase.as_str())
                                        .small()
                                        .color(phase.pip_color()),
                                );

                                if token_rate > 0.0 {
                                    ui.label(
                                        egui::RichText::new(format!("{token_rate:.0} t/s"))
                                            .small()
                                            .monospace()
                                            .color(egui::Color32::from_gray(130)),
                                    );
                                }

                                if show_close {
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui
                                                .small_button("×")
                                                .on_hover_text("Close thread")
                                                .clicked()
                                            {
                                                chosen = Some("close-thread");
                                                close_id = Some(thread_id.clone());
                                            }
                                        },
                                    );
                                }
                            });

                            // Interrupt affordance only on focused active threads.
                            if focused
                                && phase.is_active()
                                && ui
                                    .small_button("interrupt")
                                    .on_hover_text("Interrupt this agent stream")
                                    .clicked()
                            {
                                chosen = Some("interrupt");
                                state.threads[idx].phase = ThreadPhase::Idle;
                                state.threads[idx].token_rate = 0.0;
                            }

                            ui.separator();

                            let stream_h = (min_height - 48.0).max(80.0);
                            let max_h = max_height.map(|h| (h - 48.0).max(60.0));
                            let mut stream = StreamView::new(("thread", &thread_id, idx))
                                .lines(&lines)
                                .min_height(stream_h)
                                .desired_width(col_w - 16.0)
                                .monospace(true)
                                .wrap_lines(true)
                                .stick_to_bottom(true);
                            if let Some(h) = max_h {
                                stream = stream.max_height(h);
                            }
                            let _ = stream.show(ui);
                        })
                        .response;

                    // Click non-focused column chrome to switch focus lane.
                    if !focused && col_resp.clicked() {
                        chosen = Some("switch-focus");
                        state.focus(idx);
                    }

                    if idx + 1 < n {
                        ui.add_space(4.0);
                    }
                }
            });
        });

        if let Some(cid) = close_id {
            state.remove(&cid);
        }

        let mut resp = scope.response;
        if let Some(tt) = &tooltip {
            resp = resp.on_hover_text(tt.as_ref());
        }

        CanonResponse::from_egui(resp, Cow::Borrowed(IDENTITY), variant, chosen).with_id_hint(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_pair() -> ThreadDockState {
        ThreadDockState::with_threads([
            AgentThread::new("agent-a", "coder")
                .phase(ThreadPhase::Streaming)
                .lines(["line A0", "line A1"])
                .token_rate(42.0),
            AgentThread::new("agent-b", "reviewer")
                .phase(ThreadPhase::Thinking)
                .lines(["line B0"])
                .token_rate(0.0),
        ])
    }

    #[test]
    fn parallel_requires_two_threads() {
        let mut s = ThreadDockState::new();
        assert!(!s.is_parallel());
        s.push(AgentThread::new("a", "a"));
        assert!(!s.is_parallel());
        s.push(AgentThread::new("b", "b"));
        assert!(s.is_parallel());
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn streams_never_interleave_across_columns() {
        let mut s = sample_pair();
        s.append_line("agent-a", "line A2");
        s.append_line("agent-b", "line B1");
        s.append_line("agent-a", "line A3");

        let a = s.get("agent-a").unwrap();
        let b = s.get("agent-b").unwrap();
        assert_eq!(a.lines, ["line A0", "line A1", "line A2", "line A3"]);
        assert_eq!(b.lines, ["line B0", "line B1"]);
        // Structural guarantee: each column's buffer is independent.
        assert!(!a.lines.iter().any(|l| l.starts_with("line B")));
        assert!(!b.lines.iter().any(|l| l.starts_with("line A")));
    }

    #[test]
    fn focus_lane_switches_by_index_and_id() {
        let mut s = sample_pair();
        assert_eq!(s.focused_index(), 0);
        assert_eq!(s.focused().unwrap().id, "agent-a");

        s.focus(1);
        assert_eq!(s.focused().unwrap().id, "agent-b");

        assert!(s.focus_by_id("agent-a"));
        assert_eq!(s.focused().unwrap().label, "coder");

        assert!(!s.focus_by_id("missing"));
        assert_eq!(s.focused().unwrap().id, "agent-a");
    }

    #[test]
    fn set_stream_text_replaces_with_accumulated_draft() {
        let mut s = sample_pair();
        assert!(s.set_stream_text("agent-a", "hello\nworld\n"));
        let a = s.get("agent-a").unwrap();
        // Trailing newline yields a trailing empty line from split.
        assert_eq!(a.lines, ["hello", "world", ""]);
        assert!(s.set_phase("agent-a", ThreadPhase::Done));
        assert_eq!(s.get("agent-a").unwrap().phase, ThreadPhase::Done);
    }

    #[test]
    fn remove_adjusts_focus() {
        let mut s = sample_pair();
        s.push(AgentThread::new("agent-c", "tester").phase(ThreadPhase::Idle));
        s.focus(2);
        assert!(s.remove("agent-c"));
        assert_eq!(s.len(), 2);
        assert_eq!(s.focused_index(), 1);

        s.focus(1);
        assert!(s.remove("agent-a")); // index 0
        assert_eq!(s.focused_index(), 0);
        assert_eq!(s.focused().unwrap().id, "agent-b");
    }

    #[test]
    fn phase_parse_accepts_generating_alias() {
        assert_eq!(ThreadPhase::parse("generating"), Some(ThreadPhase::Streaming));
        assert_eq!(ThreadPhase::parse("awaiting"), Some(ThreadPhase::AwaitingInput));
        assert_eq!(ThreadPhase::parse("nope"), None);
        assert!(ThreadPhase::Streaming.is_active());
        assert!(!ThreadPhase::Done.is_active());
    }

    #[test]
    fn identity_uri_stable() {
        let mut s = sample_pair();
        let dock = ThreadDock::new("test", &mut s);
        assert_eq!(dock.identity_uri().as_ref(), "ui://thread-dock");
        assert_eq!(dock.affordances().len(), 3);
        assert!(!dock.mutation_axes().is_empty());
    }

    #[test]
    fn two_parallel_streams_smoke() {
        // AC: smoke with ≥2 parallel streams — pure state path.
        let mut s = ThreadDockState::new();
        s.push(
            AgentThread::new("swarm/coder", "coder")
                .phase(ThreadPhase::Streaming)
                .variant(11),
        );
        s.push(
            AgentThread::new("swarm/reviewer", "reviewer")
                .phase(ThreadPhase::Streaming)
                .variant(12),
        );
        for i in 0..5 {
            s.append_line("swarm/coder", format!("coder token {i}"));
            s.append_line("swarm/reviewer", format!("reviewer token {i}"));
        }
        assert!(s.is_parallel());
        assert_eq!(s.get("swarm/coder").unwrap().lines.len(), 5);
        assert_eq!(s.get("swarm/reviewer").unwrap().lines.len(), 5);
        assert_eq!(s.get("swarm/coder").unwrap().variant_id, 11);
        assert_eq!(s.get("swarm/reviewer").unwrap().variant_id, 12);
    }
}
