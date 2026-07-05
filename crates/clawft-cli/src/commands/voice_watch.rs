//! `weft voice watch` — the Wave 1 §W1.4 live process surface (terminal-native).
//!
//! Polls `conversation.graph` at a ~1 Hz cadence and renders each committed turn
//! node as it lands: role + text, its 4-axis classification (intent / topic /
//! emotion tier badge), and — when the turn carries a `voice_analysis` record —
//! the per-utterance decomposition rows: speaker + cosine score, the endpoint
//! fire (prob + source), a prosody summary, a moving **arousal bar** with the
//! emotion label + `source` badge, per-token confidence, and paralinguistics.
//! `--json` dumps each new node's full record (one line per node) for
//! inspection / piping.
//!
//! This is the committed-state half of the §W1.4 surface — its source is the
//! `conversation.graph` poll (ADR-067 D2), which serves `voice_analysis`
//! verbatim (Wave 1 §W1.2 wire). The **live per-frame half** ([`LiveRenderObserver`])
//! layers on top: attached to the observe-only `weft voice listen` loop, it
//! renders the partial transcript, the endpoint fire the instant it happens, and
//! the finalized turn's full decomposition as events arrive — no graph round-trip.
//! (A live capture level-meter awaits a per-frame `CaptureLevel` event; until
//! then the finalized-line + arousal bar is the fallback, plan riskiest-call #4.)

use std::collections::HashSet;
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;

use clawft_rpc::{DaemonClient, Request};
use clawft_voice_talk::{ConversationEvent, ConversationObserver};
use serde_json::Value;

/// Render width of the arousal bar.
const AROUSAL_BAR_WIDTH: usize = 24;

/// Poll `conversation.graph` for `conv_id` and stream each newly-committed turn
/// node to the terminal until Ctrl+C. `json` switches to one-record-per-line
/// JSON; `interval_ms` is the committed-state poll cadence.
pub async fn handle_watch(conv_id: String, json: bool, interval_ms: u64) -> anyhow::Result<()> {
    let mut client = DaemonClient::connect().await.ok_or_else(|| {
        anyhow::anyhow!(
            "no kernel daemon reachable — start one with `weaver kernel start` \
             (and enable [kernel.agent.classification] mode = \"keyword\")"
        )
    })?;

    if !json {
        println!(
            "=== weft voice watch — conversation `{conv_id}` \
             (poll {interval_ms}ms · Ctrl+C to exit) ===\n"
        );
        println!("Watching for committed + classified turns. Speak via `weft voice talk`,");
        println!("or record turns through agent.turn.record; each lands here as it commits.\n");
    }

    // Nodes already rendered, so a re-poll only prints turns that newly landed.
    let mut seen: HashSet<String> = HashSet::new();

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                if !json {
                    println!("\nwatch ended.");
                }
                return Ok(());
            }
            _ = tokio::time::sleep(Duration::from_millis(interval_ms)) => {
                let req = Request::with_params(
                    "conversation.graph",
                    serde_json::json!({ "conv_id": conv_id }),
                );
                match client.call(req).await {
                    Ok(resp) => match resp.into_result() {
                        Ok(val) => render_new_nodes(&val, &mut seen, json),
                        Err(e) => eprintln!("conversation.graph error: {e}"),
                    },
                    Err(e) => {
                        // One reconnect attempt; a stopped daemon ends the watch.
                        eprintln!("daemon transport error: {e}; reconnecting…");
                        match DaemonClient::connect().await {
                            Some(c) => client = c,
                            None => {
                                eprintln!("kernel daemon gone — watch stopped.");
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Print every turn node in `val` not yet in `seen` (identity nodes / stubs —
/// those without a `role` — are skipped). Nodes arrive newest-first from the
/// projection; render oldest-first so the terminal reads top-to-bottom.
fn render_new_nodes(val: &Value, seen: &mut HashSet<String>, json: bool) {
    let Some(nodes) = val.get("nodes").and_then(Value::as_array) else {
        return;
    };
    // Collect the unseen turn nodes, then reverse so commits print in order.
    let mut fresh: Vec<&Value> = Vec::new();
    for node in nodes {
        let role = node.get("role").and_then(Value::as_str).unwrap_or("");
        if role.is_empty() {
            continue; // stub / identity node — not a turn
        }
        let id = node.get("id").and_then(Value::as_str).unwrap_or("");
        if id.is_empty() || seen.contains(id) {
            continue;
        }
        seen.insert(id.to_string());
        fresh.push(node);
    }
    for node in fresh.into_iter().rev() {
        if json {
            println!("{}", serde_json::to_string(node).unwrap_or_default());
        } else {
            render_node(node);
        }
    }
}

/// Render one committed turn node as a compact multi-line block.
fn render_node(node: &Value) {
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
fn render_voice_analysis(va: &Value) {
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

// ── Live per-frame surface (§W1.4) ─────────────────────────────────────────
//
// The observe-side counterpart to the graph poll above: a `ConversationObserver`
// attached to the `weft voice listen` loop that renders the process live as
// events fire, reusing the same decomposition rows the committed-state view uses.

/// Renders the live process stream to the terminal as [`ConversationEvent`]s
/// arrive: the running partial ("listening…"), the endpoint fire the instant it
/// decides (probability, source, trailing silence), the finalized turn with its
/// full voice decomposition (speaker, prosody, arousal bar, paralinguistics), and
/// speaker enrollment plus any reply. Non-blocking — plain terminal writes.
pub struct LiveRenderObserver;

impl ConversationObserver for LiveRenderObserver {
    fn observe(&self, event: ConversationEvent) {
        match event {
            ConversationEvent::PartialTranscript { text } => {
                let line = if text.trim().is_empty() {
                    "  … listening".to_string()
                } else {
                    format!("  … {text}")
                };
                // Overwrite in place (no newline) so the partial updates live.
                print!("\r{line}\x1b[K");
                let _ = std::io::stdout().flush();
            }
            ConversationEvent::EndpointFired {
                completion_prob,
                source,
                silence_ms,
            } => {
                println!("\r  ⏎ endpoint p={completion_prob:.2} ({source}) @ {silence_ms}ms silence\x1b[K");
            }
            ConversationEvent::UserTurn {
                text,
                speaker_name,
                voice_analysis,
                ..
            } => {
                let who = speaker_name.as_deref().unwrap_or("speaker");
                println!("┌─ {who}: \u{201c}{text}\u{201d}");
                // Reuse the committed-state decomposition rows by serializing the
                // record to the same flat JSON the graph view renders from.
                if let Some(va) = voice_analysis.as_deref()
                    && let Ok(v) = serde_json::to_value(va)
                {
                    render_voice_analysis(&v);
                }
                println!("└─\n");
            }
            ConversationEvent::SpeakerEnrolled { name, .. } => {
                println!("  ✦ enrolled new speaker: {name}");
            }
            ConversationEvent::CommittedReply { answer } => {
                println!("  ↳ reply: {answer}");
            }
            // Live level meter (§W1.4 row 1). Placeholder to keep the render
            // exhaustive — coder-B owns the visual (a moving dBFS-vs-floor bar,
            // overwrite-in-place like the partial). Fields are ready to render.
            ConversationEvent::CaptureLevel {
                rms_dbfs: _,
                floor_dbfs: _,
            } => {}
            // Listen-only never fires these; a full talk loop supersedes the ack
            // with the committed reply and prunes on barge-in.
            ConversationEvent::SpeculativeReply { .. } | ConversationEvent::Interrupted => {}
        }
    }
}

/// Fan one event out to several observers. `run_live_observed` accepts a single
/// `extra_observer`, so the listen surface combines the daemon turn-recorder and
/// the [`LiveRenderObserver`] through this. Events are `Clone`, so each observer
/// gets its own copy.
pub struct FanOutObserver(pub Vec<Arc<dyn ConversationObserver>>);

impl ConversationObserver for FanOutObserver {
    fn observe(&self, event: ConversationEvent) {
        for observer in &self.0 {
            observer.observe(event.clone());
        }
    }
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
    fn render_new_nodes_dedupes_and_skips_stubs() {
        // Two turn nodes + one stub (no role); a re-poll of the same graph must
        // render nothing new.
        let graph = json!({
            "nodes": [
                { "id": "b", "role": "assistant", "state": "committed", "text": "hi" },
                { "id": "a", "role": "user", "state": "committed", "text": "hey" },
                { "id": "s", "state": "external" }
            ]
        });
        let mut seen = HashSet::new();
        render_new_nodes(&graph, &mut seen, true);
        assert_eq!(seen.len(), 2, "both turn nodes seen, stub skipped");
        // Idempotent: re-render adds nothing.
        render_new_nodes(&graph, &mut seen, true);
        assert_eq!(seen.len(), 2);
    }

    #[test]
    fn fan_out_delivers_to_every_observer() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct Counter(Arc<AtomicUsize>);
        impl ConversationObserver for Counter {
            fn observe(&self, _event: ConversationEvent) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }

        let a = Arc::new(AtomicUsize::new(0));
        let b = Arc::new(AtomicUsize::new(0));
        let fan = FanOutObserver(vec![
            Arc::new(Counter(a.clone())),
            Arc::new(Counter(b.clone())),
        ]);
        fan.observe(ConversationEvent::EndpointFired {
            completion_prob: 0.9,
            source: "smart-turn-v3".into(),
            silence_ms: 300,
        });
        assert_eq!(a.load(Ordering::SeqCst), 1, "first observer received the event");
        assert_eq!(b.load(Ordering::SeqCst), 1, "second observer received the event");
    }
}
