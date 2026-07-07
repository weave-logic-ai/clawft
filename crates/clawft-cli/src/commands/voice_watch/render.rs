//! Pure rendering helpers for `weft voice watch` — split out of `mod.rs`
//! (§W2.6) once the committed-turn view grew a second lifecycle (reply
//! attempts) alongside the original one (committed turns). No I/O beyond
//! `println!`; every line-building function is plain `&Value -> String` so it
//! can be tested directly (the `arousal_bar` idiom already established here).

use std::collections::{HashMap, HashSet};

use serde_json::Value;

/// Render width of the arousal bar.
const AROUSAL_BAR_WIDTH: usize = 24;

/// Poll-to-poll bookkeeping the watch surface needs beyond "have I printed
/// this node before": each node's last-rendered `state` (so a re-poll can
/// detect a frontier→pruned/committed transition, §W2.6), and which
/// `Contradicts` edges have already been rendered — the projection re-serves
/// every edge on every poll, so edges are deduped by `(source, target)` the
/// same way nodes are deduped by id.
#[derive(Default)]
pub(super) struct WatchState {
    node_states: HashMap<String, String>,
    seen_edges: HashSet<(String, String)>,
}

/// One node's worth of newsworthy change since the last poll: either the
/// first time it's been seen, or a change in its `state`.
enum NodeEvent<'a> {
    New(&'a Value),
    Transition {
        node: &'a Value,
        old_state: String,
        new_state: String,
    },
}

/// Render every node/edge update in one `conversation.graph` poll: a newly
/// seen turn node (once), a state transition on an already-seen node — a
/// reply attempt's frontier→pruned (interrupted) or frontier→committed
/// (finalized), or a plain turn's frontier→committed/pruned — and any
/// not-yet-rendered `Contradicts` edge (a Refine steering away from a
/// superseded attempt). Identity nodes / stubs (no `role`) are skipped.
/// Nodes arrive newest-first from the projection; events render oldest-first
/// so the terminal reads top-to-bottom.
pub(super) fn render_graph_updates(val: &Value, state: &mut WatchState, json: bool) {
    let Some(nodes) = val.get("nodes").and_then(Value::as_array) else {
        return;
    };

    let mut events: Vec<NodeEvent> = Vec::new();
    for node in nodes {
        let role = node.get("role").and_then(Value::as_str).unwrap_or("");
        if role.is_empty() {
            continue; // stub / identity node — not a turn
        }
        let id = node.get("id").and_then(Value::as_str).unwrap_or("");
        if id.is_empty() {
            continue;
        }
        let new_state = node
            .get("state")
            .and_then(Value::as_str)
            .unwrap_or("?")
            .to_string();
        match state.node_states.insert(id.to_string(), new_state.clone()) {
            None => events.push(NodeEvent::New(node)),
            Some(old_state) if old_state != new_state => {
                events.push(NodeEvent::Transition {
                    node,
                    old_state,
                    new_state,
                });
            }
            Some(_) => {} // unchanged — nothing to render
        }
    }

    for event in events.into_iter().rev() {
        match event {
            NodeEvent::New(node) => {
                if json {
                    println!("{}", serde_json::to_string(node).unwrap_or_default());
                } else if is_attempt(node) {
                    println!("{}", attempt_new_line(node));
                    println!();
                } else {
                    render_node(node);
                }
            }
            NodeEvent::Transition {
                node,
                old_state,
                new_state,
            } => {
                if json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "transition": {
                                "id": node.get("id").and_then(Value::as_str).unwrap_or(""),
                                "role": node.get("role").and_then(Value::as_str).unwrap_or(""),
                                "seq": node.get("chain_seq").and_then(Value::as_u64),
                                "old_state": old_state,
                                "new_state": new_state,
                                "attempt": is_attempt(node),
                            }
                        })
                    );
                } else {
                    println!("{}", transition_line(node, &old_state, &new_state));
                    println!();
                }
            }
        }
    }

    render_contradicts_edges(val, nodes, state, json);
}

/// Render any `Contradicts` edge not yet in `state.seen_edges` — a Refine
/// steering the conversation away from a superseded reply attempt. Seqs are
/// resolved from this poll's node list so the line reads `#new contradicts
/// #old` rather than opaque uids; falls back to the raw ids when a seq can't
/// be resolved (a far-side / evicted endpoint).
fn render_contradicts_edges(val: &Value, nodes: &[Value], state: &mut WatchState, json: bool) {
    let Some(edges) = val.get("edges").and_then(Value::as_array) else {
        return;
    };
    let seq_by_id: HashMap<&str, u64> = nodes
        .iter()
        .filter_map(|n| {
            let id = n.get("id").and_then(Value::as_str)?;
            let seq = n.get("chain_seq").and_then(Value::as_u64)?;
            Some((id, seq))
        })
        .collect();

    for edge in edges {
        if edge.get("kind").and_then(Value::as_str) != Some("Contradicts") {
            continue;
        }
        let (Some(src), Some(tgt)) = (
            edge.get("source").and_then(Value::as_str),
            edge.get("target").and_then(Value::as_str),
        ) else {
            continue;
        };
        if !state.seen_edges.insert((src.to_string(), tgt.to_string())) {
            continue; // already rendered
        }
        if json {
            println!(
                "{}",
                serde_json::json!({ "edge": { "kind": "Contradicts", "source": src, "target": tgt } })
            );
        } else {
            let new_seq = seq_by_id.get(src).copied();
            let old_seq = seq_by_id.get(tgt).copied();
            println!("{}", contradicts_line(new_seq, old_seq, src, tgt));
            println!();
        }
    }
}

/// A node carries the §W2.6 attempt marker (`register_reply_frontier`'s
/// `metadata.kind = "reply-attempt"`, projected as a flat `attempt` bool).
fn is_attempt(node: &Value) -> bool {
    node.get("attempt").and_then(Value::as_bool).unwrap_or(false)
}

/// Truncate `s` to at most `max` chars, appending an ellipsis when cut.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let head: String = s.chars().take(max).collect();
        format!("{head}…")
    }
}

/// Text for a newly-registered reply attempt (frontier, `attempt: true`) —
/// the durable busy state a register-early/commit-late reply holds while
/// generation is in flight.
fn attempt_new_line(node: &Value) -> String {
    let seq = node.get("chain_seq").and_then(Value::as_u64);
    let seq_s = seq.map(|s| format!("#{s}")).unwrap_or_default();
    let goal = node.get("goal").and_then(Value::as_str).unwrap_or("");
    format!("◇ agent busy — attempt {seq_s}: {}", truncate(goal, 60))
}

/// Text for a node's state transition. An attempt's frontier→pruned /
/// frontier→committed get their own lines (interrupted vs. finalized);
/// anything else — an attempt in some other transition, or a plain turn —
/// falls back to a generic `old→new` line so no transition is silently
/// dropped.
fn transition_line(node: &Value, old_state: &str, new_state: &str) -> String {
    let seq = node.get("chain_seq").and_then(Value::as_u64);
    let seq_s = seq.map(|s| format!("#{s}")).unwrap_or_default();
    let role = node.get("role").and_then(Value::as_str).unwrap_or("?");
    if is_attempt(node) {
        match new_state {
            "pruned" => format!("⊘ attempt {seq_s} pruned (interrupted)"),
            "committed" => format!("◆ attempt {seq_s} committed"),
            _ => format!("↦ attempt {seq_s} {old_state}→{new_state}"),
        }
    } else {
        format!("↦ turn {seq_s} {role} {old_state}→{new_state}")
    }
}

/// Text for a not-yet-rendered `Contradicts` edge (a Refine steering away
/// from a superseded attempt).
fn contradicts_line(new_seq: Option<u64>, old_seq: Option<u64>, src: &str, tgt: &str) -> String {
    match (new_seq, old_seq) {
        (Some(n), Some(o)) => format!("↯ steer: #{n} contradicts #{o}"),
        _ => format!("↯ steer: {src} contradicts {tgt}"),
    }
}

/// Render one committed turn node as a compact multi-line block.
pub(super) fn render_node(node: &Value) {
    let role = node.get("role").and_then(Value::as_str).unwrap_or("?");
    let state = node.get("state").and_then(Value::as_str).unwrap_or("?");
    let text = node.get("text").and_then(Value::as_str).unwrap_or("");
    let seq = node.get("chain_seq").and_then(Value::as_u64);

    let seq_s = seq.map(|s| format!("#{s}")).unwrap_or_default();
    println!("┌─ [{state}] {role} {seq_s}");
    if !text.is_empty() {
        println!("│  “{text}”");
    }

    // Classification: v2 dialogue act (act.refined), falling back to the legacy
    // v1 `intent` axis, + topic + the emotion provenance tier.
    if let Some(cls) = node.get("classification").filter(|v| !v.is_null()) {
        let intent = cls.get("intent").and_then(Value::as_str).unwrap_or("—");
        let act = cls
            .get("act")
            .and_then(|a| a.get("refined").and_then(Value::as_str))
            .unwrap_or(intent);
        let topic = cls.get("topic").and_then(Value::as_str).unwrap_or("—");
        let tier = cls.get("tier").and_then(Value::as_str).unwrap_or("—");
        println!("│  act={act} · topic={topic} · emotion-tier=[{tier}]");
    }

    // The rich voice decomposition, when present.
    match node.get("voice_analysis") {
        Some(va) if !va.is_null() => render_voice_analysis(va),
        _ => {}
    }
    println!("└─");
    println!();
}

/// Render the §W1.2 `voice_analysis` record rows (speaker / endpoint / prosody /
/// emotion arousal bar / paralinguistics / per-token confidence).
pub(super) fn render_voice_analysis(va: &Value) {
    // Speaker.
    if let Some(sp) = va.get("speaker") {
        let name = sp
            .get("name")
            .and_then(Value::as_str)
            .or_else(|| sp.get("id").and_then(Value::as_str))
            .unwrap_or("unknown");
        let score = sp.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        let action = sp.get("action").and_then(Value::as_str).unwrap_or("—");
        println!("│  speaker: {name} (cosine {score:.2}, {action})");
    }

    // Endpoint fire.
    if let Some(ep) = va.get("endpoint") {
        let prob = ep.get("completion_prob").and_then(Value::as_f64).unwrap_or(0.0);
        let src = ep.get("source").and_then(Value::as_str).unwrap_or("—");
        let sil = ep.get("silence_tail_ms").and_then(Value::as_u64).unwrap_or(0);
        println!("│  endpoint: ⏎ p={prob:.2} ({src}) @ {sil}ms silence");
    }

    // STT latency + per-token confidence.
    if let Some(stt) = va.get("stt") {
        let lat = stt.get("latency_ms").and_then(Value::as_u64).unwrap_or(0);
        let cmean = stt.get("token_conf_mean").and_then(Value::as_f64);
        let cmin = stt.get("token_conf_min").and_then(Value::as_f64);
        let path = stt.get("path").and_then(Value::as_str).unwrap_or("—");
        match (cmean, cmin) {
            (Some(m), Some(n)) => println!(
                "│  stt: {path} · {lat}ms · token-conf mean {m:.2} / min {n:.2}"
            ),
            _ => println!("│  stt: {path} · {lat}ms"),
        }
    }

    // Prosody summary.
    if let Some(pr) = va.get("prosody") {
        let f0 = pr.get("f0_mean_hz").and_then(Value::as_f64).unwrap_or(0.0);
        let range = pr.get("f0_range_semitones").and_then(Value::as_f64).unwrap_or(0.0);
        let rate = pr.get("rate_tokens_per_s").and_then(Value::as_f64).unwrap_or(0.0);
        let pauses = pr.get("pause_count").and_then(Value::as_u64).unwrap_or(0);
        println!(
            "│  prosody: f0 {f0:.0}Hz (±{range:.1}st) · {rate:.1} tok/s · {pauses} pause(s)"
        );
    }

    // Emotion — the moving arousal bar + label + source badge.
    if let Some(emo) = va.get("emotion") {
        let arousal = emo.get("arousal").and_then(Value::as_f64).unwrap_or(0.0);
        let valence = emo.get("valence").and_then(Value::as_f64).unwrap_or(0.0);
        let label = emo.get("label").and_then(Value::as_str).unwrap_or("—");
        let source = emo.get("source").and_then(Value::as_str).unwrap_or("—");
        let aconf = emo.get("arousal_conf").and_then(Value::as_str).unwrap_or("—");
        println!(
            "│  emotion: {} arousal {arousal:+.2} · valence {valence:+.2} · \
             {label} [{source}·{aconf}]",
            arousal_bar(arousal)
        );
    }

    // Paralinguistics (non-lexical vocalizations).
    if let Some(pl) = va.get("paralinguistics") {
        let non_lex = pl.get("non_lexical").and_then(Value::as_bool).unwrap_or(false);
        if non_lex {
            let class = pl.get("class").and_then(Value::as_str).unwrap_or("unknown");
            println!("│  paralinguistics: non-lexical → {class}");
        }
    }

    // Capture health (SNR) — a one-line honesty check on the audio.
    if let Some(audio) = va.get("audio") {
        let snr = audio.get("snr_db").and_then(Value::as_f64);
        let clip = audio.get("clip_pct").and_then(Value::as_f64).unwrap_or(0.0);
        if let Some(snr) = snr {
            println!("│  capture: SNR {snr:.1}dB · clip {clip:.1}%");
        }
    }
}

/// A fixed-width arousal meter. Arousal is `[-1, 1]` (0.5 ≈ neutral per the
/// keyword floor); map to `[0, 1]` for the bar and mark the filled portion.
fn arousal_bar(arousal: f64) -> String {
    let norm = ((arousal.clamp(-1.0, 1.0) + 1.0) / 2.0).clamp(0.0, 1.0);
    let filled = (norm * AROUSAL_BAR_WIDTH as f64).round() as usize;
    let filled = filled.min(AROUSAL_BAR_WIDTH);
    let mut bar = String::with_capacity(AROUSAL_BAR_WIDTH + 2);
    bar.push('[');
    for i in 0..AROUSAL_BAR_WIDTH {
        bar.push(if i < filled { '█' } else { '·' });
    }
    bar.push(']');
    bar
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn arousal_bar_scales_low_to_high() {
        // Low arousal → mostly empty; high → mostly full; identical width.
        let low = arousal_bar(-1.0);
        let mid = arousal_bar(0.0);
        let high = arousal_bar(1.0);
        assert_eq!(low.chars().count(), high.chars().count());
        let fill = |s: &str| s.chars().filter(|c| *c == '█').count();
        assert!(fill(&low) < fill(&mid), "low < mid arousal fill");
        assert!(fill(&mid) < fill(&high), "mid < high arousal fill");
        assert_eq!(fill(&high), AROUSAL_BAR_WIDTH, "arousal 1.0 fills the bar");
    }

    #[test]
    fn render_graph_updates_dedupes_and_skips_stubs() {
        // Two turn nodes + one stub (no role); a re-poll of the same graph must
        // track nothing new (no state changed).
        let graph = json!({
            "nodes": [
                { "id": "b", "role": "assistant", "state": "committed", "text": "hi" },
                { "id": "a", "role": "user", "state": "committed", "text": "hey" },
                { "id": "s", "state": "external" }
            ]
        });
        let mut watch = WatchState::default();
        render_graph_updates(&graph, &mut watch, true);
        assert_eq!(watch.node_states.len(), 2, "both turn nodes tracked, stub skipped");
        // Idempotent: re-render tracks nothing new.
        render_graph_updates(&graph, &mut watch, true);
        assert_eq!(watch.node_states.len(), 2);
    }

    #[test]
    fn render_graph_updates_renders_new_attempt_and_tracks_frontier() {
        let graph = json!({
            "nodes": [
                {
                    "id": "att1", "role": "assistant", "state": "frontier",
                    "attempt": true, "goal": "what's the weather", "chain_seq": 5
                }
            ]
        });
        let mut watch = WatchState::default();
        render_graph_updates(&graph, &mut watch, true);
        assert_eq!(watch.node_states.get("att1"), Some(&"frontier".to_string()));
        // Re-poll of the same frontier state is idempotent.
        render_graph_updates(&graph, &mut watch, true);
        assert_eq!(watch.node_states.len(), 1);
    }

    #[test]
    fn render_graph_updates_detects_pruned_transition() {
        let mut watch = WatchState::default();
        let frontier = json!({
            "nodes": [
                { "id": "att1", "role": "assistant", "state": "frontier", "attempt": true, "goal": "g", "chain_seq": 7 }
            ]
        });
        render_graph_updates(&frontier, &mut watch, true);

        let pruned = json!({
            "nodes": [
                { "id": "att1", "role": "assistant", "state": "pruned", "attempt": true, "goal": "g", "chain_seq": 7 }
            ]
        });
        render_graph_updates(&pruned, &mut watch, true);
        assert_eq!(watch.node_states.get("att1"), Some(&"pruned".to_string()), "transition tracked");

        // Re-poll of the same pruned state renders (tracks) nothing new.
        render_graph_updates(&pruned, &mut watch, true);
        assert_eq!(watch.node_states.len(), 1);
    }

    #[test]
    fn render_graph_updates_detects_committed_transition() {
        let mut watch = WatchState::default();
        let frontier = json!({
            "nodes": [
                { "id": "att2", "role": "assistant", "state": "frontier", "attempt": true, "goal": "g", "chain_seq": 3 }
            ]
        });
        render_graph_updates(&frontier, &mut watch, true);

        let committed = json!({
            "nodes": [
                { "id": "att2", "role": "assistant", "state": "committed", "attempt": true, "goal": "g", "chain_seq": 3 }
            ]
        });
        render_graph_updates(&committed, &mut watch, true);
        assert_eq!(watch.node_states.get("att2"), Some(&"committed".to_string()));
    }

    #[test]
    fn render_graph_updates_detects_non_attempt_transition() {
        // A plain (non-attempt) frontier node renders once with today's
        // behavior, but a later state change must still be tracked/rendered.
        let mut watch = WatchState::default();
        let frontier = json!({
            "nodes": [ { "id": "t1", "role": "assistant", "state": "frontier", "text": "hi" } ]
        });
        render_graph_updates(&frontier, &mut watch, true);
        assert_eq!(watch.node_states.get("t1"), Some(&"frontier".to_string()));

        let committed = json!({
            "nodes": [ { "id": "t1", "role": "assistant", "state": "committed", "text": "hi" } ]
        });
        render_graph_updates(&committed, &mut watch, true);
        assert_eq!(watch.node_states.get("t1"), Some(&"committed".to_string()), "non-attempt transition tracked too");
    }

    #[test]
    fn render_graph_updates_dedupes_contradicts_edges() {
        let mut watch = WatchState::default();
        let graph = json!({
            "nodes": [
                { "id": "new1", "role": "assistant", "state": "frontier", "attempt": true, "goal": "amended", "chain_seq": 9 },
                { "id": "old1", "role": "assistant", "state": "pruned", "attempt": true, "goal": "orig", "chain_seq": 8 }
            ],
            "edges": [
                { "source": "new1", "target": "old1", "kind": "Contradicts", "weight": 1.0, "chain_seq": 9 }
            ]
        });
        render_graph_updates(&graph, &mut watch, true);
        assert_eq!(watch.seen_edges.len(), 1, "edge tracked once");
        assert!(watch.seen_edges.contains(&("new1".to_string(), "old1".to_string())));
        // Re-poll of the same graph doesn't re-track the edge.
        render_graph_updates(&graph, &mut watch, true);
        assert_eq!(watch.seen_edges.len(), 1, "idempotent — edge not double counted");
    }

    #[test]
    fn attempt_new_line_truncates_long_goal() {
        let node = json!({ "chain_seq": 5, "goal": "a".repeat(80) });
        let line = attempt_new_line(&node);
        assert!(line.starts_with("◇ agent busy — attempt #5: "), "line was: {line}");
        assert!(line.contains('…'), "long goal truncated with ellipsis");
    }

    #[test]
    fn attempt_new_line_leaves_short_goal_untouched() {
        let node = json!({ "chain_seq": 2, "goal": "what's the weather" });
        let line = attempt_new_line(&node);
        assert_eq!(line, "◇ agent busy — attempt #2: what's the weather");
    }

    #[test]
    fn transition_line_renders_attempt_pruned_and_committed() {
        let node = json!({ "chain_seq": 4, "role": "assistant", "attempt": true });
        assert_eq!(
            transition_line(&node, "frontier", "pruned"),
            "⊘ attempt #4 pruned (interrupted)"
        );
        assert_eq!(
            transition_line(&node, "frontier", "committed"),
            "◆ attempt #4 committed"
        );
    }

    #[test]
    fn transition_line_renders_generic_for_non_attempt() {
        let node = json!({ "chain_seq": 2, "role": "user" });
        let line = transition_line(&node, "frontier", "committed");
        assert!(line.contains("#2") && line.contains("frontier") && line.contains("committed"));
    }

    #[test]
    fn contradicts_line_resolves_seqs_when_available() {
        assert_eq!(
            contradicts_line(Some(9), Some(8), "new1", "old1"),
            "↯ steer: #9 contradicts #8"
        );
        assert_eq!(
            contradicts_line(None, None, "new1", "old1"),
            "↯ steer: new1 contradicts old1"
        );
    }
}
