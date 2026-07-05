//! Session hydration from the [`ConversationSink`] (M3 store collapse, P3).
//!
//! This is the read half of the store collapse (design
//! `.planning/hermes-loop/m3-store-collapse-design.md` §D2): the in-memory
//! [`Session`] that [`ContextBuilder`](super::context::ContextBuilder) reads is
//! no longer independently persisted by `SessionManager`. Instead it is
//! **reconstructed each turn** from the single durable store — the
//! [`ConversationSink`] turn log — and the assembly stage is left
//! byte-for-byte untouched.
//!
//! # The superset/subset asymmetry (the load-bearing rule)
//!
//! The sink turn log is a **superset** (design §1): it holds every tool-loop
//! intermediate — assistant turns that emitted `tool_calls`, and `tool` result
//! turns — that the retired `SessionManager` store never recorded.
//! [`filter_store1_subset`] reproduces `SessionManager`'s exact record subset:
//! `user` turns plus the **final** `assistant` reply of each exchange, dropping
//! the intermediates. Reconstructing that subset is what keeps the assembled
//! prompt identical to the pre-collapse path — the guarantee the
//! `prompt_identity_golden` gate proves.
//!
//! [`filter_store1_subset`] and [`hydrate_session`] are the **production**
//! entry points the golden's cutover test exercises directly; the golden's
//! byte-identical assertion is the gate that they reproduce store 1.

use std::collections::HashMap;

use clawft_types::session::Session;

use super::sink::{ConversationSink, Turn};

/// The design's D2 hydration rule: reconstruct `SessionManager`'s (store 1's)
/// exact subset from the sink **superset**.
///
/// Keeps `user` turns and the **final** `assistant` reply of each exchange;
/// drops the tool-loop intermediates the superset additionally carries —
/// assistant turns that invoked tools (`tool_calls` set) and `tool` result
/// turns (`tool_call_id` set). A non-final assistant turn is, by construction,
/// exactly a `tool_calls`-bearing turn (`run_tool_loop` only appends an
/// assistant turn with `tool_calls` when it is about to dispatch tools and
/// loop again), so filtering on the two tool markers is sufficient to keep
/// only the final assistant of each exchange.
///
/// Order is preserved. Returns borrows into `turns` — the caller (typically
/// [`hydrate_session`]) copies the `role`/`content` it needs.
pub fn filter_store1_subset(turns: &[Turn]) -> Vec<&Turn> {
    turns
        .iter()
        .filter(|t| t.tool_calls.is_none() && t.tool_call_id.is_none())
        .collect()
}

/// Reconstruct the in-memory assembly [`Session`] for `conv_id` from the sink
/// (design §D2 + §D3).
///
/// Reads the sink **superset** (`window == 0` → all turns, so the filter and
/// the downstream `Session::get_history(memory_window)` re-window see the full
/// record — see the `ConversationSink::history` P1 contract), applies
/// [`filter_store1_subset`], and replays the surviving turns into a fresh
/// `Session` via `add_message` — the exact shape the pre-collapse
/// `SessionManager` path produced. The per-conversation metadata sidecar
/// (design §D3, home of the hallucination score) is read from
/// [`ConversationSink::meta`] into `Session::metadata`; a `Null` / non-object
/// sidecar yields empty metadata, so a downstream hallucination-score lookup
/// defaults to `0.0` exactly as before.
///
/// Infallible by contract: the sink read degrades to an empty history / `Null`
/// meta on any error (design §6), so hydration never fails a turn.
pub async fn hydrate_session(sink: &dyn ConversationSink, conv_id: &str, window: usize) -> Session {
    let turns = sink.history(conv_id, window).await;
    let mut session = Session::new(conv_id);
    for turn in filter_store1_subset(&turns) {
        session.add_message(&turn.role, &turn.content, None);
    }
    if let serde_json::Value::Object(map) = sink.meta(conv_id).await {
        session.metadata = map.into_iter().collect::<HashMap<_, _>>();
    }
    session
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::sink::InMemorySink;
    use serde_json::json;

    fn user(content: &str) -> Turn {
        Turn {
            turn_id: format!("u-{content}"),
            role: "user".into(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            ts_ms: 1_700_000_000_000,
            voice_analysis: None,
        }
    }

    fn assistant(content: &str) -> Turn {
        Turn {
            turn_id: format!("a-{content}"),
            role: "assistant".into(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            voice_analysis: None,
            ts_ms: 1_700_000_000_000,
        }
    }

    fn assistant_tool_call(content: &str) -> Turn {
        Turn {
            tool_calls: Some(vec![json!({"id": "c1", "name": "search"})]),
            ..assistant(content)
        }
    }

    fn tool_result(content: &str) -> Turn {
        Turn {
            role: "tool".into(),
            tool_call_id: Some("c1".into()),
            ..user(content)
        }
    }

    /// T3: the filter drops both intermediate kinds and keeps user + final
    /// assistant, in order — the same conversation the golden records.
    #[test]
    fn filter_drops_tool_intermediates_keeps_user_and_final_assistant() {
        let superset = vec![
            user("What is the capital of France?"),
            assistant("The capital of France is Paris."),
            user("What's the weather in Paris right now?"),
            assistant_tool_call("Let me check the current conditions."),
            tool_result("Paris: 18°C, clear skies, wind 6 km/h."),
            assistant("It's currently 18°C and clear in Paris."),
            user("Thanks, that's all."),
            assistant("You're welcome! Happy to help."),
        ];

        let subset = filter_store1_subset(&superset);
        let shape: Vec<(&str, &str)> = subset
            .iter()
            .map(|t| (t.role.as_str(), t.content.as_str()))
            .collect();
        assert_eq!(
            shape,
            vec![
                ("user", "What is the capital of France?"),
                ("assistant", "The capital of France is Paris."),
                ("user", "What's the weather in Paris right now?"),
                ("assistant", "It's currently 18°C and clear in Paris."),
                ("user", "Thanks, that's all."),
                ("assistant", "You're welcome! Happy to help."),
            ],
        );
        assert!(
            subset
                .iter()
                .all(|t| t.tool_calls.is_none() && t.tool_call_id.is_none()),
            "no intermediate should survive the filter"
        );
    }

    /// Hydration replays the store-1 subset into a `Session` whose message
    /// (role, content) sequence matches what the pre-collapse path produced.
    #[tokio::test]
    async fn hydrate_reconstructs_store1_subset() {
        let sink = InMemorySink::new();
        for t in [
            user("hi"),
            assistant_tool_call("checking"),
            tool_result("42"),
            assistant("the answer is 42"),
        ] {
            sink.append_turn("c:1", t).await.unwrap();
        }

        let session = hydrate_session(&sink, "c:1", 0).await;
        let roles_contents: Vec<(String, String)> = session
            .messages
            .iter()
            .map(|m| {
                (
                    m["role"].as_str().unwrap_or_default().to_string(),
                    m["content"].as_str().unwrap_or_default().to_string(),
                )
            })
            .collect();
        assert_eq!(
            roles_contents,
            vec![
                ("user".into(), "hi".into()),
                ("assistant".into(), "the answer is 42".into()),
            ]
        );
    }

    /// T5 (part): the metadata sidecar hydrates into `Session::metadata`.
    #[tokio::test]
    async fn hydrate_reads_metadata_sidecar() {
        let sink = InMemorySink::new();
        sink.append_turn("c:1", user("hi")).await.unwrap();
        sink.set_meta("c:1", json!({"hallucination_score": 0.42}))
            .await;

        let session = hydrate_session(&sink, "c:1", 0).await;
        assert_eq!(
            session.metadata.get("hallucination_score").and_then(|v| v.as_f64()),
            Some(0.42)
        );
    }

    /// T5 (part): an absent sidecar yields empty metadata (score defaults 0.0).
    #[tokio::test]
    async fn hydrate_absent_metadata_is_empty() {
        let sink = InMemorySink::new();
        sink.append_turn("c:1", user("hi")).await.unwrap();

        let session = hydrate_session(&sink, "c:1", 0).await;
        assert!(session.metadata.is_empty());
        assert_eq!(
            session
                .metadata
                .get("hallucination_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            0.0
        );
    }
}
