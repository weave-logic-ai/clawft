//! Conversation-end postmortem (ADR-058 Phase 5, deferred step 4).
//!
//! When a conversation ends ([`agent.chat.end`](crate::daemon)), the L2
//! [`SessionTier`](clawft_service_agent::SessionTier) view must be rolled up
//! before it is dropped — else "fresh each run" becomes "amnesia each run". The
//! durable fact promoted to the trunk is synthesized by a short LLM postmortem:
//! we hand the conversation digest to the same [`LlmClient`] the daemon already
//! uses for `agent.chat` and ask it to distill what is worth remembering.
//!
//! This leg is **live** (it makes a real model call), so the integration test
//! for it is `#[ignore]`'d against the Hermes :8090 endpoint; the wiring it
//! feeds (`AgentService::end_conversation` → `SessionTier::promote_and_drop`) is
//! covered deterministically with a Mock in `clawft-service-agent`'s tests.

use clawft_service_llm::{ChatMessage, LlmClient};
use tracing::{debug, warn};

/// Sentinel the model returns when the conversation held nothing durable.
const NO_DURABLE_FACT: &str = "NONE";

/// System instruction for the distiller. Deliberately terse and grounded: only
/// promote facts the conversation actually established, never speculation.
const POSTMORTEM_SYSTEM: &str = "You are a memory distiller for an AI agent. \
Given a conversation transcript, extract ONLY the durable facts worth \
remembering for future conversations: stable preferences, identities, \
decisions, and commitments. Ignore transient chatter, greetings, and \
tool mechanics. Reply with one or two sentences of plain durable facts, or \
exactly NONE if nothing is worth keeping.";

/// Synthesize the durable fact for a finished conversation.
///
/// Returns `Some(fact)` when the model distilled something worth promoting, or
/// `None` when the digest was empty, the model replied with the `NONE` sentinel,
/// or the call failed (best-effort — a postmortem failure must never break the
/// `agent.chat.end` path; the view is dropped without promotion instead).
pub async fn summarize_durable_facts(llm: &LlmClient, digest: &str) -> Option<String> {
    let digest = digest.trim();
    if digest.is_empty() {
        debug!("conv postmortem: empty digest, nothing to distill");
        return None;
    }

    let messages = vec![
        ChatMessage::system(POSTMORTEM_SYSTEM),
        ChatMessage::user(format!("Conversation transcript:\n\n{digest}")),
    ];

    // Deterministic (temperature 0) and short — this is a distillation, not a
    // generation. The cap keeps the round-trip cheap on conversation close.
    let resp = match llm.complete(messages, Some(0.0), Some(256)).await {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "conv postmortem: LLM call failed; dropping view without promotion");
            return None;
        }
    };

    let fact = resp
        .choices
        .first()
        .map(|c| c.message.content.as_text().trim().to_string())
        .unwrap_or_default();

    if fact.is_empty() || fact.eq_ignore_ascii_case(NO_DURABLE_FACT) {
        debug!(fact = %fact, "conv postmortem: nothing durable to promote");
        None
    } else {
        Some(fact)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_durable_sentinel_is_recognized() {
        // Pure-logic guard for the sentinel handling the live call relies on
        // (the live LLM round-trip itself is the #[ignore]'d test below).
        assert!(NO_DURABLE_FACT.eq_ignore_ascii_case("none"));
    }

    /// Live postmortem against Hermes :8090 (ADR-060). #[ignore]'d: needs the
    /// serving endpoint up. Run with:
    /// `cargo test -p clawft-weave --features <daemon set> -- --ignored live_postmortem`
    #[tokio::test]
    #[ignore = "needs live Hermes :8090 (ADR-060)"]
    async fn live_postmortem_distills_durable_fact() {
        let llm = LlmClient::new(clawft_service_llm::LlmConfig {
            base_url: "http://127.0.0.1:8090".to_string(),
            model: "hermes-4.3-36b".to_string(),
            ..Default::default()
        })
        .expect("llm client");
        let digest = "User: my name is Mat and I prefer Rust over Go.\n\
                      Assistant: Noted, Mat.\n";
        let fact = summarize_durable_facts(&llm, digest).await;
        let fact = fact.expect("expected a durable fact from a fact-bearing transcript");
        let lc = fact.to_lowercase();
        assert!(
            lc.contains("mat") || lc.contains("rust"),
            "distilled fact should mention the durable detail: {fact}"
        );
    }
}
