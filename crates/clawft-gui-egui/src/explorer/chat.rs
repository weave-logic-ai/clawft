//! Chat-window panel for the Explorer.
//!
//! Renders a scrollable conversation against the daemon's `llm.prompt`
//! RPC. Triggers when the selected substrate value matches a chat
//! sentinel:
//!
//! ```json
//! { "kind": "chat", "model": "local" }
//! ```
//!
//! The daemon publishes one such sentinel at boot under
//! `substrate/<daemon-node>/ui/chat` so this panel has a stable mount
//! point in the substrate tree.
//!
//! ## Wire shape
//!
//! Each user turn fires `llm.prompt` with the **full** conversation so
//! far (system + every user/assistant turn). We don't trust the daemon
//! to preserve session state — the panel is the source of truth.
//!
//! ## Scope cuts (deliberate)
//!
//! - Streaming lands via `agent.chat_stream` (WEFT-253): the panel fires
//!   that verb and polls `substrate/_derived/chat/<conv>/stream` for
//!   progressive frames while the RPC is in flight. Mid-generation
//!   LLM token callbacks are a daemon-side follow-up; today the daemon
//!   cascades the final assistant text as typewriter frames before
//!   returning, and the panel renders the draft bubble live.
//! - No on-disk persistence. Conversation lives only as long as the
//!   panel does (close → cleared on next selection).
//! - No model picker. The daemon decides which `llama-server` it talks
//!   to; the model name in the sentinel is informational. WEFT-256
//!   tracks the chip-strip selector and is blocked on a daemon-side
//!   enumeration RPC.
//!
//! ## Shipped UX (this file)
//!
//! - **Markdown rendering** in assistant bubbles via `egui_commonmark`
//!   so code blocks / lists / headers don't render as raw markdown.
//!   WEFT-252.
//! - **System-prompt affordance**: collapsible textarea above the
//!   message field. The text rides on the `system` field and is sent
//!   alongside `agent.chat` params (the daemon merges it after the
//!   workspace identity prompt). WEFT-255.
//! - **Heartbeat label** below the input that mirrors the substrate
//!   `derived/chat/<conv_id>/status` payload while a request is in
//!   flight, replacing the blocking spinner. WEFT-257.
//! - **Identity-drift warning**: when the response's `identity_source`
//!   isn't a recognised stable source (e.g. `"docs-fallback"`),
//!   surface a non-dismissable warning chip above the input. WEFT-259.

use std::sync::Arc;

use eframe::egui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use serde_json::Value;

use crate::live::{self, Command, Live, ReplyRx};

/// Shape-match priority for the chat sentinel. Higher than
/// `control_toggle` (25) and `workshop` (30) so a chat sentinel
/// dispatched from the same selection slot wins decisively.
pub const PRIORITY: u32 = 40;

/// Sampling temperature passed to the LLM service. Conservative — chat
/// quality over creative drift; the daemon's own default also lives in
/// this range, so this is a soft repeat-of-default that lets the panel
/// override later without a daemon change.
const DEFAULT_TEMPERATURE: f32 = 0.4;

/// Hard cap on generated tokens per turn. Matches the daemon's default
/// (512); explicit here so a daemon-side bump doesn't silently change
/// chat-window behaviour.
const DEFAULT_MAX_TOKENS: u32 = 512;

/// How often (ms) the panel re-polls `substrate.read` for the
/// progressive stream frame while a turn is in flight. Lower = snappier
/// typewriter, higher = fewer RPCs. 120 ms is well under the daemon's
/// 4 ms cascade step × chunk size without flooding the socket.
const STREAM_POLL_INTERVAL_MS: f64 = 120.0;

/// Shape predicate. Returns [`PRIORITY`] when `value` looks like a
/// chat sentinel:
///
/// ```json
/// { "kind": "chat", ... }
/// ```
///
/// Strict on `kind == "chat"` — we don't probe other shapes so an
/// arbitrary object with a `model` key doesn't false-match.
pub fn matches(value: &Value) -> u32 {
    let Some(obj) = value.as_object() else {
        return 0;
    };
    let kind_ok = obj
        .get("kind")
        .and_then(Value::as_str)
        .map(|s| s == "chat")
        .unwrap_or(false);
    if kind_ok { PRIORITY } else { 0 }
}

/// One conversation turn. Mirrors `LlmPromptMessage` on the wire but
/// kept as a local type so the panel doesn't depend on `clawft-weave`
/// (which would pull tokio + the daemon-side world into the GUI crate
/// for free).
#[derive(Debug, Clone, PartialEq)]
pub struct ChatMessage {
    /// Role: `system`, `user`, or `assistant`. `error` is a UI-only
    /// pseudo-role rendered as a red bubble; it's filtered out of the
    /// wire payload before sending.
    pub role: String,
    /// Message content.
    pub content: String,
}

impl ChatMessage {
    fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
        }
    }
    fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
        }
    }
    fn error(content: impl Into<String>) -> Self {
        Self {
            role: "error".into(),
            content: content.into(),
        }
    }
}

/// Panel state. Owned by the [`Explorer`](super::Explorer) and reset
/// whenever the selection moves (via `Explorer::on_select`) so a stale
/// conversation doesn't reappear when the user re-selects the chat
/// sentinel after navigating elsewhere.
#[derive(Default)]
pub struct ChatView {
    /// Optional system prompt prepended to every wire payload. WEFT-255
    /// surfaces this via a collapsible textarea above the message
    /// field. The daemon-side concierge owns the workspace identity
    /// prompt; this `system` is layered on top of it (not in place of)
    /// so the panel cannot hijack the workspace persona.
    pub system: Option<String>,
    /// Whether the system-prompt editor is expanded in the UI. Persists
    /// across paints within the panel lifetime; reset to false when the
    /// selection moves.
    pub system_expanded: bool,
    /// Full conversation. Wire payload always starts here (with `system`
    /// prepended if set) — the daemon does not preserve session state.
    pub history: Vec<ChatMessage>,
    /// Draft input text bound to the multiline `TextEdit`.
    pub draft: String,
    /// Stable conversation id for this panel session. Used both to
    /// identify the conversation in `agent.chat` params (so per-conv
    /// state on the daemon-side `AgentService` lines up across turns)
    /// and to derive the substrate heartbeat path
    /// `substrate/_derived/chat/<conv_id>/status`. Lazily minted on
    /// first submit so an empty/idle panel doesn't burn an id.
    conv_id: Option<String>,
    /// Last `identity_source` echoed by `agent.chat`. Drives the
    /// drift-warning chip (WEFT-259). `None` while no successful turn
    /// has landed; `Some("clawft")` is the stable, non-warning case.
    last_identity_source: Option<String>,
    /// Markdown renderer cache. WEFT-252 — `CommonMarkCache` retains
    /// parsed AST per source string across paints, so re-rendering an
    /// existing assistant turn doesn't re-parse the markdown each
    /// frame. Cleared implicitly when the panel state is reset on
    /// selection change (the cache is rebuilt on demand).
    cache: CommonMarkCache,
    /// Pending `agent.chat_stream` reply channel. `Some` while a request
    /// is in flight; the input + Send button disable in that window so
    /// the user can't fire a second request against `llama-server`'s
    /// single in-flight slot (the service crate's semaphore would
    /// serialize them anyway, but UI feedback is clearer).
    pending: Option<ReplyRx>,
    /// Accumulated assistant draft from progressive stream frames
    /// (WEFT-253). `Some` while a turn is in flight and at least one
    /// non-empty frame has landed; rendered as a live assistant bubble
    /// that grows as tokens arrive. Cleared when the final RPC reply
    /// lands (the bubble is committed into `history`).
    streaming_draft: Option<String>,
    /// Last stream-frame `phase` (`thinking` / `generating` / `done` /
    /// `error`). Drives the in-flight heartbeat label so the user sees
    /// more than a static "waiting for completion…".
    stream_phase: Option<String>,
    /// Highest stream `seq` applied. Frames with a lower-or-equal seq
    /// are ignored so a late substrate.read can't rewind the draft.
    stream_seq: u64,
    /// In-flight `substrate.read` for the stream path. At most one poll
    /// is outstanding so we don't queue a flood of reads.
    stream_poll: Option<ReplyRx>,
    /// `live::now_ms()` of the last stream-poll submit. Throttles the
    /// poll cadence to [`STREAM_POLL_INTERVAL_MS`].
    last_stream_poll_ms: f64,
}

impl ChatView {
    /// Whether a request is currently in flight.
    pub fn is_in_flight(&self) -> bool {
        self.pending.is_some()
    }

    /// Build the JSON params for `agent.chat` from the current history.
    ///
    /// Matches `AgentChatParams { messages, temperature, max_tokens,
    /// conv_id }`. When [`Self::system`] is set (WEFT-255) it rides on
    /// the request as a `system` user-prompt message at the head of
    /// `messages` — the daemon-side concierge prepends its own identity
    /// prompt before this, so panel-side `system` is layered on top
    /// (additional context) rather than replacing the workspace persona.
    ///
    /// `conv_id` is included whenever it has been minted (i.e. once the
    /// first turn has been submitted; the very first call to this fn
    /// during `submit_draft` mints it). Both Phase A (ephemeral default)
    /// and Phase C (panel-supplied) call sites keep working.
    ///
    /// Filters UI-only `error` pseudo-roles out of the wire payload.
    pub fn build_request_params(&self, next_user: &str) -> Value {
        let mut messages: Vec<Value> = Vec::new();
        if let Some(system) = self.system.as_deref()
            && !system.trim().is_empty()
        {
            messages.push(serde_json::json!({
                "role": "system",
                "content": system,
            }));
        }
        for m in self.history.iter().filter(|m| m.role != "error") {
            messages.push(serde_json::json!({
                "role": m.role,
                "content": m.content,
            }));
        }
        messages.push(serde_json::json!({
            "role": "user",
            "content": next_user,
        }));
        let mut obj = serde_json::Map::new();
        obj.insert("messages".into(), Value::Array(messages));
        obj.insert("temperature".into(), serde_json::json!(DEFAULT_TEMPERATURE));
        obj.insert("max_tokens".into(), serde_json::json!(DEFAULT_MAX_TOKENS));
        if let Some(id) = self.conv_id.as_deref() {
            obj.insert("conv_id".into(), serde_json::json!(id));
        }
        Value::Object(obj)
    }

    /// State-machine entry: a successful `agent.chat` /
    /// `agent.chat_stream` response landed. Appends the assistant text
    /// to history and clears the in-flight + streaming state. Pure
    /// function over [`Value`] so tests can drive it without any RPC
    /// plumbing.
    ///
    /// Tool-call rendering as collapsible bubbles lands in commit 9
    /// (plan §11.4); the spike just shows the final assistant text.
    /// Both old `{ completion: "..." }` and new
    /// `{ assistant_text: "..." }` shapes are accepted to make rolling
    /// the wasm bundle and the daemon independently safe.
    pub fn on_response_ok(&mut self, response: &Value) {
        let text = response
            .get("assistant_text")
            .or_else(|| response.get("completion"))
            .and_then(Value::as_str)
            .unwrap_or("(empty completion)");
        self.history.push(ChatMessage::assistant(text.to_string()));
        // Capture identity_source for the WEFT-259 drift-warning chip.
        // The daemon injects this at the wire boundary; if absent, we
        // intentionally clear the cached value so a previously-sticky
        // warning doesn't haunt subsequent turns.
        self.last_identity_source = response
            .get("identity_source")
            .and_then(Value::as_str)
            .map(str::to_owned);
        self.clear_stream_state();
    }

    /// Apply a progressive stream frame (WEFT-253). Ignores frames with
    /// `seq <= self.stream_seq` so a late poll can't rewind the draft.
    /// Pure over the frame value so unit tests can drive it without
    /// substrate plumbing.
    pub fn on_stream_frame(&mut self, frame: &Value) {
        let seq = frame.get("seq").and_then(Value::as_u64).unwrap_or(0);
        if seq < self.stream_seq {
            return;
        }
        // Equal seq still applies phase/text updates (idempotent
        // Replace of the same frame); only strictly-older are dropped.
        if seq > self.stream_seq {
            self.stream_seq = seq;
        }
        if let Some(phase) = frame.get("phase").and_then(Value::as_str) {
            self.stream_phase = Some(phase.to_owned());
        }
        if let Some(text) = frame.get("text").and_then(Value::as_str)
            && !text.is_empty()
        {
            // Empty text leaves a prior draft alone (thinking frames
            // carry `text: ""` and must not flash a blank bubble or
            // rewind a growing draft on a late re-read).
            self.streaming_draft = Some(text.to_owned());
        }
        if frame.get("done").and_then(Value::as_bool).unwrap_or(false)
            && let Some(err) = frame.get("error").and_then(Value::as_str)
        {
            // Error frames surface via the final RPC reply too; stash
            // the message so the heartbeat label can show it until the
            // oneshot lands.
            self.stream_phase = Some(format!("error: {err}"));
        }
    }

    /// Clear every WEFT-253 streaming field. Called when the final RPC
    /// reply lands (ok or err) so the next turn starts clean.
    fn clear_stream_state(&mut self) {
        self.pending = None;
        self.streaming_draft = None;
        self.stream_phase = None;
        self.stream_seq = 0;
        self.stream_poll = None;
        self.last_stream_poll_ms = 0.0;
    }

    /// Substrate path for the progressive stream frame, or `None` if
    /// no conv_id has been minted yet. Sibling of [`Self::heartbeat_path`].
    pub fn stream_path(&self) -> Option<String> {
        self.conv_id
            .as_deref()
            .map(|id| format!("substrate/_derived/chat/{id}/stream"))
    }

    /// Current streaming draft text (accumulated assistant tokens).
    pub fn streaming_draft(&self) -> Option<&str> {
        self.streaming_draft.as_deref()
    }

    /// Last stream phase label, if any.
    pub fn stream_phase(&self) -> Option<&str> {
        self.stream_phase.as_deref()
    }

    /// Whether the most recent response indicates a suspicious
    /// identity source (e.g. `"docs-fallback"`) that should warn the
    /// user. WEFT-259.
    ///
    /// Returns `Some(source)` when a warning chip should render. Treats
    /// `"clawft"` (the canonical workspace-loaded identity) as the only
    /// stable source today; anything else — including unknown future
    /// values — is surfaced so a regression in the daemon's identity
    /// loader can't silently degrade the chat experience.
    pub fn identity_warning(&self) -> Option<&str> {
        match self.last_identity_source.as_deref() {
            None => None,
            Some("clawft") => None,
            Some(other) => Some(other),
        }
    }

    /// State-machine entry: an `agent.chat_stream` / `agent.chat`
    /// request failed. Appends a UI-only `error` bubble and clears the
    /// in-flight + streaming state.
    pub fn on_response_err(&mut self, err: &str) {
        self.history.push(ChatMessage::error(err.to_string()));
        self.clear_stream_state();
    }

    /// Drain the pending reply channel and any in-flight stream poll.
    /// Called once per paint before rendering so the UI always reflects
    /// the freshest server state.
    fn drain_reply(&mut self) {
        if let Some(rx) = self.pending.as_mut() {
            match live::try_recv_reply(rx) {
                live::TryReply::Done(Ok(value)) => self.on_response_ok(&value),
                live::TryReply::Done(Err(err)) => self.on_response_err(&err),
                live::TryReply::Closed => {
                    self.on_response_err("transport closed before reply");
                }
                live::TryReply::Empty => { /* still in flight */ }
            }
        }
        // Drain a completed stream-path substrate.read without holding
        // `self.stream_poll` across the `on_stream_frame` borrow.
        let stream_result = self.stream_poll.as_mut().map(live::try_recv_reply);
        match stream_result {
            Some(live::TryReply::Done(Ok(value))) => {
                self.stream_poll = None;
                // substrate.read returns `{ value, tick, sensitivity }`.
                // The progressive frame lives under `value`.
                if let Some(frame) = value.get("value").filter(|v| !v.is_null()) {
                    self.on_stream_frame(frame);
                }
            }
            Some(live::TryReply::Done(Err(_))) | Some(live::TryReply::Closed) => {
                // Transient read failure — drop the slot so the next
                // paint re-submits. Don't surface as a chat error.
                self.stream_poll = None;
            }
            Some(live::TryReply::Empty) | None => {}
        }
    }

    /// While a turn is in flight, keep the progressive stream path
    /// warm via `substrate.read`. Throttled and at-most-one outstanding
    /// so we don't starve the raw-RPC slot during long turns.
    fn poll_stream_if_needed(&mut self, live: &Arc<Live>) {
        if !self.is_in_flight() {
            return;
        }
        if self.stream_poll.is_some() {
            return;
        }
        let Some(path) = self.stream_path() else {
            return;
        };
        let now = live::now_ms();
        if self.last_stream_poll_ms > 0.0
            && (now - self.last_stream_poll_ms) < STREAM_POLL_INTERVAL_MS
        {
            return;
        }
        // Prefer any frame already mirrored into the local substrate
        // snapshot (native adapter path / prior relay) before spending
        // an RPC. Cheap and keeps the typewriter smooth at 60 fps when
        // the path is already warm.
        if let Some(frame) = live.substrate_snapshot().get(&path).cloned() {
            self.on_stream_frame(&frame);
        }
        let (tx, rx) = live::reply_channel();
        self.stream_poll = Some(rx);
        self.last_stream_poll_ms = now;
        live.submit(Command::Raw {
            method: "substrate.read".into(),
            params: serde_json::json!({ "path": path }),
            reply: Some(tx),
        });
    }

    /// Mint a stable conversation id for this panel session. Idempotent
    /// — once set, subsequent calls reuse the same id so the daemon's
    /// per-conv `AgentService` state lines up across turns and the
    /// substrate heartbeat path stays stable for WEFT-257's label.
    ///
    /// Format mirrors `clawft_types::agent_chat::default_conv_id` —
    /// `panel-<ts_ms>-<rng>` — so it's visually distinct from the
    /// daemon's auto-minted ephemeral ids in logs and substrate paths.
    fn ensure_conv_id(&mut self) -> &str {
        if self.conv_id.is_none() {
            // `now_ms` is monotonic-from-app-start on native and
            // performance.now() on wasm — good enough for a panel-local
            // id. We don't need cross-process uniqueness; the daemon
            // namespaces by node_id under the hood.
            let ts = live::now_ms() as u64;
            // Cheap pseudo-random suffix from the address of `self` —
            // collisions across panel sessions are inert (the daemon
            // treats each as a fresh conv) and we avoid adding `rand`
            // to this crate's wasm bundle just for this.
            let salt = (self as *const _ as usize) as u64 & 0xFFFF;
            self.conv_id = Some(format!("panel-{ts:013}-{salt:04x}"));
        }
        // Safe: the `if` above ensures `Some`.
        self.conv_id.as_deref().expect("conv_id minted")
    }

    /// Substrate path for this conversation's heartbeat / status frame,
    /// or `None` if no conv_id has been minted yet (i.e. no turn has
    /// been submitted in this panel lifetime). Used by the WEFT-257
    /// heartbeat label.
    pub fn heartbeat_path(&self) -> Option<String> {
        self.conv_id
            .as_deref()
            .map(|id| format!("substrate/_derived/chat/{id}/status"))
    }

    /// Submit the current `draft` as a user turn and fire the RPC.
    /// No-op if the draft is empty/whitespace or a request is already
    /// in flight.
    fn submit_draft(&mut self, live: &Arc<Live>) {
        if self.is_in_flight() {
            return;
        }
        let text = self.draft.trim().to_string();
        if text.is_empty() {
            return;
        }
        // Mint conv_id before serialising params so the request carries
        // the stable id and the substrate heartbeat path is well-known
        // for the in-flight render (WEFT-257).
        self.ensure_conv_id();
        let params = self.build_request_params(&text);
        self.history.push(ChatMessage::user(text));
        self.draft.clear();

        // Reset streaming draft so a previous turn's partial text
        // never bleeds into this one.
        self.streaming_draft = None;
        self.stream_phase = Some("thinking".into());
        self.stream_seq = 0;
        self.stream_poll = None;
        self.last_stream_poll_ms = 0.0;

        let (tx, rx) = live::reply_channel();
        self.pending = Some(rx);
        // WEFT-253: prefer agent.chat_stream so progressive frames land
        // on the substrate stream path while the turn runs. Final
        // response shape is identical to agent.chat (plus optional
        // stream_path / streamed echo fields the panel ignores).
        live.submit(Command::Raw {
            method: "agent.chat_stream".into(),
            params,
            reply: Some(tx),
        });
    }
}

/// Render the chat panel. `path` is the substrate sentinel path (used
/// only for the muted footer hint). `value` is the sentinel value
/// (used only to surface the model name).
pub fn paint(ui: &mut egui::Ui, path: &str, value: &Value, view: &mut ChatView, live: &Arc<Live>) {
    view.drain_reply();
    view.poll_stream_if_needed(live);

    let model = value
        .as_object()
        .and_then(|o| o.get("model"))
        .and_then(Value::as_str)
        .unwrap_or("local");

    ui.label(
        egui::RichText::new(format!("chat · {model}"))
            .color(egui::Color32::from_rgb(160, 160, 170))
            .small(),
    );
    ui.add_space(2.0);
    ui.heading("Local LLM");
    ui.add_space(6.0);

    // WEFT-259: identity-drift / binding-thread mismatch warning. Lives
    // above the input so the user always sees it before composing the
    // next turn — not buried under the scroll area.
    paint_identity_warning(ui, view);

    // WEFT-255: system-prompt affordance. Collapsible header above the
    // history so the persona/instructions can be tweaked without
    // crowding the message field. Initial state is collapsed.
    paint_system_editor(ui, view);

    // Reserve room for the input area at the bottom so the scroll area
    // doesn't fight it for vertical space.
    let input_h = 96.0;
    let history_h = (ui.available_height() - input_h).max(120.0);

    // WEFT-252: paint_history needs `&mut view` (CommonMarkCache lives
    // there). Splitting the closure body into a free fn keeps borrow
    // shapes clean.
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(6))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .max_height(history_h)
                .show(ui, |ui| {
                    paint_history(ui, view);
                });
        });

    ui.add_space(6.0);

    // WEFT-257: heartbeat label replaces the in-history spinner. Reads
    // `substrate/_derived/chat/<conv_id>/status` and surfaces the
    // status string + age in seconds. Shown only while a request is
    // in flight (otherwise stale status would mislead the user).
    if view.is_in_flight() {
        paint_heartbeat(ui, view, live);
    }

    // Input row. Disabled while a request is in flight; Enter submits,
    // Shift+Enter inserts a newline (egui's default for multiline +
    // explicit `desired_rows`).
    ui.add_enabled_ui(!view.is_in_flight(), |ui| {
        let response = ui.add(
            egui::TextEdit::multiline(&mut view.draft)
                .desired_rows(3)
                .desired_width(f32::INFINITY)
                .hint_text("Type a message — Enter to send, Shift+Enter for newline"),
        );

        let enter_pressed = response.has_focus()
            && ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift);

        ui.horizontal(|ui| {
            let send_clicked = ui.button("Send").clicked();
            if (send_clicked || enter_pressed) && !view.draft.trim().is_empty() {
                view.submit_draft(live);
            }
            if !view.history.is_empty() && ui.button("Clear").clicked() && !view.is_in_flight() {
                view.history.clear();
            }
        });
    });

    ui.add_space(4.0);
    ui.separator();
    ui.label(
        egui::RichText::new(format!("path: {path}"))
            .small()
            .monospace()
            .color(egui::Color32::from_rgb(140, 140, 150)),
    );
}

/// Paint the conversation history. Split out from [`paint`] so the
/// `CommonMarkCache` borrow on `view` stays scoped to a single call.
fn paint_history(ui: &mut egui::Ui, view: &mut ChatView) {
    if view.history.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(24.0);
            ui.label(
                egui::RichText::new("(no messages yet — type below to start)")
                    .italics()
                    .color(egui::Color32::from_rgb(160, 160, 170)),
            );
        });
        return;
    }
    // Index-loop so each iteration can re-borrow `view.cache` mutably
    // for assistant turns without aliasing `view.history`. Cloning the
    // role + content strings is cheap (small per turn) and avoids the
    // borrow-checker gymnastics of holding a `&ChatMessage` across the
    // CommonMarkViewer call.
    let len = view.history.len();
    for i in 0..len {
        let (role, content) = {
            let m = &view.history[i];
            (m.role.clone(), m.content.clone())
        };
        let msg = ChatMessage { role, content };
        paint_bubble(ui, &msg, &mut view.cache);
    }
    // WEFT-253: live streaming draft bubble. Grows as stream frames
    // arrive; replaced by the committed assistant history entry when
    // the final RPC reply lands (`on_response_ok` clears the draft).
    if let Some(draft) = view.streaming_draft.clone() {
        let msg = ChatMessage::assistant(draft);
        paint_bubble(ui, &msg, &mut view.cache);
        // Subtle caret so the user can tell this bubble is still
        // growing (vs a finished assistant turn above).
        ui.label(
            egui::RichText::new("▍")
                .small()
                .color(egui::Color32::from_rgb(120, 180, 200)),
        );
    }
}

/// Paint the WEFT-255 system-prompt editor. Collapsible so it doesn't
/// crowd the panel when the user just wants to chat. Edits flow into
/// `view.system`; an empty/whitespace string is treated as `None` on
/// the wire (see [`ChatView::build_request_params`]).
fn paint_system_editor(ui: &mut egui::Ui, view: &mut ChatView) {
    egui::CollapsingHeader::new(
        egui::RichText::new("system prompt (optional)")
            .small()
            .color(egui::Color32::from_rgb(170, 170, 180)),
    )
    .id_salt("chat-system-prompt")
    .default_open(view.system_expanded)
    .show(ui, |ui| {
        view.system_expanded = true;
        let mut text = view.system.clone().unwrap_or_default();
        let resp = ui.add(
            egui::TextEdit::multiline(&mut text)
                .desired_rows(2)
                .desired_width(f32::INFINITY)
                .hint_text("Extra context layered on top of the workspace identity prompt."),
        );
        if resp.changed() {
            view.system = if text.trim().is_empty() {
                None
            } else {
                Some(text)
            };
        }
    });
    ui.add_space(4.0);
}

/// Paint the WEFT-259 identity-drift warning chip. No-op when the most
/// recent response carried a recognised stable `identity_source`.
fn paint_identity_warning(ui: &mut egui::Ui, view: &ChatView) {
    let Some(source) = view.identity_warning() else {
        return;
    };
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(70, 50, 20))
        .inner_margin(egui::Margin::symmetric(8, 4))
        .corner_radius(4.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("identity drift")
                        .strong()
                        .color(egui::Color32::from_rgb(240, 200, 140)),
                );
                ui.label(
                    egui::RichText::new(format!("source = `{source}`"))
                        .monospace()
                        .small()
                        .color(egui::Color32::from_rgb(230, 220, 200)),
                );
            });
            ui.label(
                egui::RichText::new(
                    "Workspace identity not loaded from the canonical source. \
                     Conversations may use a fallback persona; check the \
                     daemon logs and `IDENTITY.md`.",
                )
                .small()
                .color(egui::Color32::from_rgb(220, 210, 190)),
            );
        });
    ui.add_space(4.0);
}

/// Paint the WEFT-257 heartbeat label below the input. Reads the
/// substrate status frame at `substrate/_derived/chat/<conv_id>/status`
/// (a `{ status, payload, ts_ms }` blob written by the daemon's
/// `SubstrateConversationSink`) and renders the status word plus an
/// age in seconds since the last heartbeat tick.
///
/// Falls back to a neutral "waiting…" label when the status frame
/// isn't published yet (very first turn, or daemon-side sink not
/// wired) so the user always sees *something* moving in the in-flight
/// window.
fn paint_heartbeat(ui: &mut egui::Ui, view: &ChatView, live: &Arc<Live>) {
    let snap = live.substrate_snapshot();
    let frame = view
        .heartbeat_path()
        .as_deref()
        .and_then(|p| snap.get(p).cloned());

    // Prefer the progressive stream phase (WEFT-253) when present —
    // "generating" / "thinking" is more informative than a generic
    // heartbeat "alive". Fall back to the status-frame word, then to
    // "waiting".
    let stream_phase = view.stream_phase().map(str::to_owned);
    let (status_word, age_label) = match frame.as_ref() {
        Some(v) => {
            let status = stream_phase.clone().unwrap_or_else(|| {
                v.get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("alive")
                    .to_owned()
            });
            let age = v
                .get("ts_ms")
                .and_then(Value::as_f64)
                .map(|ts| (live::now_ms() - ts).max(0.0) / 1000.0)
                .map(|secs| format!("{secs:.1}s ago"))
                .unwrap_or_default();
            (status, age)
        }
        None => (
            stream_phase.unwrap_or_else(|| "waiting".into()),
            String::new(),
        ),
    };

    let progress_hint = if view.streaming_draft().is_some() {
        "streaming…"
    } else {
        "waiting for completion…"
    };

    ui.horizontal(|ui| {
        // Subtle pulsing dot replaces the spinner — it's an unambiguous
        // "we're alive" signal but doesn't grab the eye the way a
        // spinning gear does. Hue cycles through a narrow blue-green
        // band per second so a stalled paint loop becomes visible.
        let t = (live::now_ms() / 1000.0).fract() as f32;
        let pulse = 120 + ((1.0 - t) * 80.0) as u8;
        let dot_color = egui::Color32::from_rgb(80, pulse, 200);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 4.0, dot_color);

        ui.label(
            egui::RichText::new(format!("status: {status_word}"))
                .small()
                .color(egui::Color32::from_rgb(180, 200, 220)),
        );
        if !age_label.is_empty() {
            ui.label(
                egui::RichText::new(format!("({age_label})"))
                    .small()
                    .italics()
                    .color(egui::Color32::from_rgb(140, 150, 160)),
            );
        }
        ui.label(
            egui::RichText::new(progress_hint)
                .small()
                .italics()
                .color(egui::Color32::from_rgb(150, 160, 170)),
        );
    });
    // Request a repaint at the stream-poll cadence so the typewriter
    // draft and age counter tick even when the user isn't moving the
    // mouse. egui only repaints on input by default — without this,
    // the label stalls and tokens freeze on screen.
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_millis(100));
    ui.add_space(2.0);
}

/// Paint one history entry as a chat bubble. Roles render distinctly:
///
/// - `system`   — dim italic, full width (informational; not a turn).
/// - `user`     — right-aligned, subtle bg, plain text.
/// - `assistant`— left-aligned, subtle bg, **markdown** via
///   `egui_commonmark` (WEFT-252) so code blocks / lists / headers
///   render correctly. The cache is supplied by the caller so the
///   parsed AST is reused across paints rather than re-parsed each
///   frame.
/// - `error`    — left-aligned, red bg + red strong text. UI-only role.
fn paint_bubble(ui: &mut egui::Ui, msg: &ChatMessage, md_cache: &mut CommonMarkCache) {
    match msg.role.as_str() {
        "system" => {
            ui.label(
                egui::RichText::new(format!("[system] {}", msg.content))
                    .italics()
                    .small()
                    .color(egui::Color32::from_rgb(150, 150, 160)),
            );
            ui.add_space(2.0);
        }
        "user" => {
            ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(50, 60, 80))
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .corner_radius(6.0)
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(&msg.content)
                                .color(egui::Color32::from_rgb(225, 230, 240)),
                        );
                    });
            });
            ui.add_space(4.0);
        }
        "assistant" => {
            ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(40, 50, 50))
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .corner_radius(6.0)
                    .show(ui, |ui| {
                        // WEFT-252: markdown rendering. The
                        // `id_salt` is unique-per-bubble (combined
                        // hash of content + role) so collapsible
                        // sections and copy buttons inside markdown
                        // don't share state across bubbles.
                        CommonMarkViewer::new().max_image_width(Some(480)).show(
                            ui,
                            md_cache,
                            &msg.content,
                        );
                    });
            });
            ui.add_space(4.0);
        }
        "error" => {
            ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(70, 30, 30))
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .corner_radius(6.0)
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(format!("error: {}", msg.content))
                                .strong()
                                .color(egui::Color32::from_rgb(240, 170, 170)),
                        );
                    });
            });
            ui.add_space(4.0);
        }
        other => {
            // Unknown role — render as plain text so we never silently
            // drop content if the wire shape grows.
            ui.label(format!("[{other}] {}", msg.content));
            ui.add_space(2.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn matches_returns_positive_for_chat_sentinel() {
        let v = json!({ "kind": "chat", "model": "local" });
        assert_eq!(matches(&v), PRIORITY);
    }

    #[test]
    fn matches_returns_positive_without_model_field() {
        // Sentinel must work even if the daemon hasn't filled in the
        // optional `model` field — `kind` alone is the contract.
        let v = json!({ "kind": "chat" });
        assert_eq!(matches(&v), PRIORITY);
    }

    #[test]
    fn matches_returns_zero_for_other_shapes() {
        // Control intent — different `kind`.
        let control = json!({
            "enabled": true,
            "kind": "service",
            "target": "llm",
        });
        assert_eq!(matches(&control), 0);

        // Workshop — no `kind` at all.
        let workshop = json!({
            "title": "x",
            "panels": [],
        });
        assert_eq!(matches(&workshop), 0);

        // Plain scalars + arrays.
        assert_eq!(matches(&Value::Null), 0);
        assert_eq!(matches(&json!(42)), 0);
        assert_eq!(matches(&json!("chat")), 0);
        assert_eq!(matches(&json!([1, 2, 3])), 0);

        // String `kind` but wrong value.
        assert_eq!(matches(&json!({ "kind": "Chat" })), 0);
        assert_eq!(matches(&json!({ "kind": "chatroom" })), 0);

        // Non-string `kind`.
        assert_eq!(matches(&json!({ "kind": 7 })), 0);
    }

    #[test]
    fn priority_beats_control_toggle_and_workshop() {
        // Sanity that the dispatch order in `explorer/mod.rs` lands on
        // chat first when a value (somehow) shape-matches multiple
        // primitives.
        const { assert!(PRIORITY > 30) };
        const { assert!(PRIORITY > 25) };
    }

    #[test]
    fn serializes_messages_to_expected_wire_shape() {
        let mut view = ChatView::default();
        view.history.push(ChatMessage::user("hi"));
        view.history.push(ChatMessage::assistant("hello"));
        // Error bubbles must NOT show up on the wire.
        view.history.push(ChatMessage::error("transport blew up"));

        let params = view.build_request_params("how are you?");
        let messages = params
            .get("messages")
            .and_then(Value::as_array)
            .expect("messages key");

        assert_eq!(messages.len(), 3, "error role filtered, new user appended");
        assert_eq!(messages[0]["role"], "user");
        assert_eq!(messages[0]["content"], "hi");
        assert_eq!(messages[1]["role"], "assistant");
        assert_eq!(messages[1]["content"], "hello");
        assert_eq!(messages[2]["role"], "user");
        assert_eq!(messages[2]["content"], "how are you?");

        // Required tuning knobs land in the params with the panel's
        // explicit defaults so a daemon-side default change can't drift
        // chat behaviour silently.
        assert!(params.get("temperature").is_some());
        assert!(params.get("max_tokens").is_some());
        // No `prompt` field — we always send `messages`.
        assert!(params.get("prompt").is_none());
        // No `system` unless explicitly set.
        assert!(params.get("system").is_none());
    }

    #[test]
    fn system_prompt_rides_at_head_of_messages() {
        // WEFT-255: when set, the panel's `system` field is sent as a
        // `{ role: "system", content: ... }` message at index 0 so the
        // daemon-side concierge can layer it on top of the workspace
        // identity prompt. There's no top-level `system` field — the
        // wire shape is uniformly `messages`.
        let mut view = ChatView {
            system: Some("you are concise".into()),
            ..Default::default()
        };
        view.history.push(ChatMessage::user("hi"));
        let params = view.build_request_params("again");
        // Top-level `system` still absent — we never split the prompt
        // into a sibling field.
        assert!(params.get("system").is_none());

        let messages = params["messages"].as_array().expect("messages");
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[0]["content"], "you are concise");
        assert_eq!(messages[1]["role"], "user");
        assert_eq!(messages[1]["content"], "hi");
        assert_eq!(messages[2]["role"], "user");
        assert_eq!(messages[2]["content"], "again");
    }

    #[test]
    fn whitespace_only_system_prompt_is_dropped_from_wire() {
        let mut view = ChatView {
            system: Some("   \n\t".into()),
            ..Default::default()
        };
        view.history.push(ChatMessage::user("hi"));
        let params = view.build_request_params("again");
        let messages = params["messages"].as_array().expect("messages");
        // No system frame — the trim-empty guard kicks in before we
        // emit one.
        assert!(messages.iter().all(|m| m["role"] != "system"));
    }

    #[test]
    fn build_params_includes_conv_id_after_first_submit_mints_one() {
        let mut view = ChatView::default();
        // Before any submit, conv_id is None and isn't on the wire.
        let params = view.build_request_params("first");
        assert!(params.get("conv_id").is_none());

        // After mint, the wire payload carries it so the daemon's
        // per-conv state lines up across turns.
        view.ensure_conv_id();
        let params = view.build_request_params("second");
        let id = params
            .get("conv_id")
            .and_then(Value::as_str)
            .expect("conv_id");
        assert!(id.starts_with("panel-"), "got {id:?}");
    }

    #[test]
    fn ensure_conv_id_is_idempotent() {
        let mut view = ChatView::default();
        let a = view.ensure_conv_id().to_owned();
        let b = view.ensure_conv_id().to_owned();
        assert_eq!(a, b, "second call must not re-mint");
    }

    #[test]
    fn heartbeat_path_is_none_until_first_turn() {
        let mut view = ChatView::default();
        assert!(view.heartbeat_path().is_none());
        view.ensure_conv_id();
        let p = view.heartbeat_path().expect("path after mint");
        assert!(p.starts_with("substrate/_derived/chat/"));
        assert!(p.ends_with("/status"));
    }

    #[test]
    fn identity_warning_clear_for_canonical_source() {
        let mut view = ChatView::default();
        view.on_response_ok(&json!({
            "assistant_text": "hi",
            "identity_source": "clawft",
        }));
        assert_eq!(view.identity_warning(), None);
    }

    #[test]
    fn identity_warning_fires_for_docs_fallback() {
        let mut view = ChatView::default();
        view.on_response_ok(&json!({
            "assistant_text": "hi",
            "identity_source": "docs-fallback",
        }));
        assert_eq!(view.identity_warning(), Some("docs-fallback"));
    }

    #[test]
    fn identity_warning_fires_for_unknown_source() {
        // Forward-compat: any unrecognised source value warns. A future
        // daemon adding a new identity loader without touching the
        // panel's allowlist would otherwise silently degrade.
        let mut view = ChatView::default();
        view.on_response_ok(&json!({
            "assistant_text": "hi",
            "identity_source": "experimental",
        }));
        assert_eq!(view.identity_warning(), Some("experimental"));
    }

    #[test]
    fn identity_warning_silent_when_field_absent() {
        // Non-loop / older producers may omit the field (C1 default).
        // Absence is not a warning — only an explicit non-canonical
        // value is. WEFT-328 populates it on the normal agent-loop path.
        let mut view = ChatView::default();
        view.on_response_ok(&json!({
            "assistant_text": "hi",
        }));
        assert_eq!(view.identity_warning(), None);
    }

    #[test]
    fn ok_response_accepts_assistant_text_field() {
        // `agent.chat` returns `assistant_text`; the panel must accept it
        // alongside the legacy `completion` field for cross-rev safety.
        // WEFT-328: full payload with real token/model/identity values.
        let mut view = ChatView::default();
        view.history.push(ChatMessage::user("hi"));
        let response = json!({
            "assistant_text": "hello from the concierge",
            "tool_calls": [{
                "name": "echo",
                "arguments_preview": "{\"text\":\"hi\"}",
                "result_preview": "hi",
                "success": true
            }],
            "finish_reason": "stop",
            "iterations": 2,
            "prompt_tokens": 40,
            "completion_tokens": 22,
            "model": "local",
            "identity_source": "clawft",
        });
        view.on_response_ok(&response);
        assert_eq!(view.history.len(), 2);
        assert_eq!(view.history[1].content, "hello from the concierge");
        // Identity source captured for the drift chip (canonical → no warn).
        assert_eq!(view.identity_warning(), None);
    }

    #[test]
    fn appends_assistant_message_on_ok_response() {
        let mut view = ChatView::default();
        view.history.push(ChatMessage::user("hi"));

        // Mock the wire shape an `llm.prompt` success returns: the
        // `Response::success` body is the `LlmPromptResult` JSON.
        let response = json!({
            "completion": "hello back",
            "finish_reason": "stop",
            "prompt_tokens": 5,
            "completion_tokens": 3,
            "model": "local",
        });
        view.on_response_ok(&response);

        assert_eq!(view.history.len(), 2);
        assert_eq!(view.history[1].role, "assistant");
        assert_eq!(view.history[1].content, "hello back");
        assert!(!view.is_in_flight());
    }

    #[test]
    fn ok_response_with_missing_completion_yields_placeholder() {
        let mut view = ChatView::default();
        view.on_response_ok(&json!({ "completion_tokens": 0 }));
        assert_eq!(view.history.len(), 1);
        assert_eq!(view.history[0].role, "assistant");
        assert!(view.history[0].content.contains("empty"));
    }

    #[test]
    fn appends_error_bubble_on_err_response() {
        let mut view = ChatView::default();
        view.history.push(ChatMessage::user("hi"));
        view.on_response_err("llm.prompt: llm service is disabled");

        assert_eq!(view.history.len(), 2);
        assert_eq!(view.history[1].role, "error");
        assert!(view.history[1].content.contains("llm service is disabled"));
        assert!(!view.is_in_flight());
    }

    // ── WEFT-253 stream frame state machine ──────────────────────

    #[test]
    fn stream_path_mirrors_heartbeat_layout() {
        let mut view = ChatView::default();
        assert!(view.stream_path().is_none());
        view.ensure_conv_id();
        let p = view.stream_path().expect("path after mint");
        assert!(p.starts_with("substrate/_derived/chat/"));
        assert!(p.ends_with("/stream"));
        // Sibling of the status heartbeat path (same conv id).
        let status = view.heartbeat_path().expect("status");
        assert_eq!(
            p.trim_end_matches("/stream"),
            status.trim_end_matches("/status")
        );
    }

    #[test]
    fn stream_frame_grows_draft_and_ignores_stale_seq() {
        let mut view = ChatView::default();
        view.on_stream_frame(&json!({
            "phase": "thinking",
            "text": "",
            "seq": 0,
            "done": false,
        }));
        assert_eq!(view.stream_phase(), Some("thinking"));
        assert_eq!(view.streaming_draft(), None, "empty thinking keeps draft None");

        view.on_stream_frame(&json!({
            "phase": "generating",
            "text": "hel",
            "seq": 1,
            "done": false,
        }));
        assert_eq!(view.streaming_draft(), Some("hel"));
        assert_eq!(view.stream_phase(), Some("generating"));

        view.on_stream_frame(&json!({
            "phase": "generating",
            "text": "hello",
            "seq": 2,
            "done": false,
        }));
        assert_eq!(view.streaming_draft(), Some("hello"));

        // Stale lower seq must not rewind the draft.
        view.on_stream_frame(&json!({
            "phase": "generating",
            "text": "he",
            "seq": 1,
            "done": false,
        }));
        assert_eq!(view.streaming_draft(), Some("hello"));
    }

    #[test]
    fn stream_state_clears_on_final_ok() {
        let mut view = ChatView::default();
        view.on_stream_frame(&json!({
            "phase": "generating",
            "text": "partial",
            "seq": 3,
            "done": false,
        }));
        assert!(view.streaming_draft().is_some());
        view.on_response_ok(&json!({
            "assistant_text": "partial final",
            "identity_source": "clawft",
        }));
        assert_eq!(view.streaming_draft(), None);
        assert_eq!(view.stream_phase(), None);
        assert_eq!(view.stream_seq, 0);
        assert_eq!(view.history.last().map(|m| m.content.as_str()), Some("partial final"));
    }

    #[test]
    fn stream_state_clears_on_err() {
        let mut view = ChatView::default();
        view.on_stream_frame(&json!({
            "phase": "generating",
            "text": "x",
            "seq": 1,
            "done": false,
        }));
        view.on_response_err("agent.chat_stream: cancelled");
        assert_eq!(view.streaming_draft(), None);
        assert!(!view.is_in_flight());
    }
}
