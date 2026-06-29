//! Tests for the L2 [`SessionTier`] (ADR-058 Phase 5). Deterministic: a Mock
//! embedder + an in-memory `ChainManager`, no live model. Split to a sibling
//! file for the <500-line rule.

use super::*;
use clawft_kernel::chain::ChainManager;
use clawft_kernel::embedding::MockEmbeddingProvider;
use serde_json::json;

fn make_tier(dims: usize) -> (SessionTier, Arc<ChainManager>) {
    let embedder: Arc<dyn EmbeddingProvider> = Arc::new(MockEmbeddingProvider::new(dims));
    let chain = Arc::new(ChainManager::new(0, 1000));
    (SessionTier::new(embedder, chain.clone(), None), chain)
}

/// Append a turn to the chain (as the substrate sink does) and index it into
/// the tier; returns the assigned chain sequence (the witness).
async fn append_and_index(tier: &SessionTier, chain: &ChainManager, conv: &str, text: &str) -> u64 {
    let ev = chain.append(
        "agent",
        "agent.chat.turn",
        Some(json!({ "conv": conv, "content": text })),
    );
    tier.index_turn(conv, ev.sequence, "agent.chat.turn", text)
        .await;
    ev.sequence
}

#[tokio::test]
async fn index_then_graft_recalls_with_witness() {
    let (tier, chain) = make_tier(64);
    let s = append_and_index(&tier, &chain, "c1", "the launch code is ZULU-7").await;

    let block = tier.graft_block("c1", "the launch code is ZULU-7").await;
    assert_eq!(block.len(), 1);
    let body = &block[0].content;
    assert!(
        body.contains("ZULU-7"),
        "graft should recall the fact: {body}"
    );
    assert!(
        body.contains(&format!("chain_seq {s}")),
        "graft should cite the witness seq: {body}"
    );
}

#[tokio::test]
async fn graft_empty_for_unknown_conversation() {
    let (tier, _chain) = make_tier(64);
    assert!(tier.graft_block("never-seen", "q").await.is_empty());
}

/// HEADLINE (ADR-058 Phase 5.2): a fact stated early, pruned from the live
/// window, is re-grafted later in the SAME session WITH its witness chain.
///
/// Deterministic — a Mock embedder makes embeddings content-addressed, so a
/// content-identical re-query exercises the index→prune→regraft machinery
/// without a live model. (Semantic, non-identical recall needs the live Qwen3
/// embedder; that path is covered by the #[ignore]'d live tests.)
#[tokio::test]
async fn pruned_fact_is_regrafted_with_witness_chain() {
    let (tier, chain) = make_tier(64);
    let conv = "recall-session";
    let fact = "my favorite color is teal";

    // Turn 1: state the fact early — appended to the chain (witness) + indexed.
    let s_fact = append_and_index(&tier, &chain, conv, fact).await;

    // ... several later turns grow the conversation ...
    for i in 0..4 {
        append_and_index(
            &tier,
            &chain,
            conv,
            &format!("unrelated chatter number {i}"),
        )
        .await;
    }

    // Window overflow: prune to the single most-recent live chunk. The early
    // fact is evicted (Stale) but stays on the chain and in the index.
    let pruned = tier.prune_to_recent(conv, 1);
    assert!(pruned >= 1, "older chunks should be pruned");

    // Later in the SAME session: recall the fact. Despite being pruned from the
    // live window, it is re-grafted via retrieval.
    let block = tier.graft_block(conv, fact).await;
    assert_eq!(block.len(), 1);
    let body = &block[0].content;
    assert!(
        body.contains("teal"),
        "pruned fact must be recalled: {body}"
    );
    assert!(
        body.contains(&format!("chain_seq {s_fact}")),
        "must cite the witness seq {s_fact}: {body}"
    );

    // The witness chain is verifiable: the cited sequence is a real
    // agent.chat.turn event on the chain.
    let witness = chain
        .tail_from(s_fact - 1)
        .into_iter()
        .find(|e| e.sequence == s_fact)
        .expect("witness event present on chain");
    assert_eq!(witness.kind, "agent.chat.turn");
}

#[tokio::test]
async fn promote_emits_memory_promote_and_drops_view() {
    let (tier, chain) = make_tier(64);
    let conv = "promote-session";
    let s = append_and_index(&tier, &chain, conv, "remember the api base is https://x").await;
    assert!(tier.mark_important(conv, s));

    let promoted_seq = tier
        .promote_and_drop(conv, "api base = https://x")
        .expect("promotion event emitted");

    // The promotion is a real memory.promote event on the chain (lineage).
    let ev = chain
        .tail_from(promoted_seq - 1)
        .into_iter()
        .find(|e| e.sequence == promoted_seq)
        .expect("memory.promote event present");
    assert_eq!(ev.kind, "memory.promote");

    // View dropped after promotion: a subsequent graft finds nothing.
    assert!(tier.graft_block(conv, "api base").await.is_empty());
}

#[tokio::test]
async fn prune_noop_when_under_keep() {
    let (tier, chain) = make_tier(64);
    append_and_index(&tier, &chain, "c", "only one").await;
    assert_eq!(tier.prune_to_recent("c", 5), 0);
}

// ── 5.3 budget validation ────────────────────────────────────────────

/// Per-turn graft hot-path budget (embed + scoped HNSW search), against the
/// **real** ADR-059 Qwen3 embedder.
///
/// Justification (measured, release, M-series CPU; weights staged + onnx-embeddings):
/// - warm Qwen3 embed: ~22ms (the model loads eagerly in the provider ctor, so
///   the per-call embed is the dominant per-turn cost, not a cold start);
/// - per-turn graft with a built index: ~22ms; the lone HNSW graph build at the
///   32-chunk threshold (`HNSW_THRESHOLD`) costs ~35ms, the worst single turn;
/// - bulk-building 64 chunks at once (an artifact of the old test shape, never
///   the per-turn path) was ~142ms release / ~2.6s debug — this is the build
///   cost, NOT the steady-state query, and is why this test now warms first.
///
/// 50ms gives ~1.4x headroom over the measured 35ms worst turn.
const GRAFT_HOT_PATH_BUDGET_MS: u128 = 50;

/// ADR-058 Phase 5.3 — per-turn graft latency budget against the real embedder.
///
/// #[ignore]'d: needs the Qwen3 weights (staged out-of-tree, gitignored) AND the
/// `clawft-kernel/onnx-embeddings` feature; without both, `select_embedding_provider`
/// degrades to the Mock and the measurement is meaningless. Run live with:
/// `cargo test -p clawft-service-agent --features clawft-kernel/onnx-embeddings \
///   -- --ignored budget_graft_latency`
///
/// The hot path measured is the **per-turn** query against an already-built
/// index — production warms the embedder at daemon startup ([`SessionTier::warm`])
/// and the per-conversation index builds incrementally as turns accumulate, so a
/// turn never pays the one-shot bulk build. This test mirrors that: pre-warm the
/// embedder, then warm the index with one throwaway graft, then time a graft.
#[tokio::test]
#[ignore = "needs real Qwen3 weights + clawft-kernel/onnx-embeddings feature"]
async fn budget_graft_latency_under_hot_path() {
    use clawft_kernel::embedding::select_embedding_provider;
    use std::time::Instant;

    // Real embedder when weights + feature are present; degrades to Mock
    // otherwise (the #[ignore] gate means this only runs intentionally).
    let embedder: Arc<dyn EmbeddingProvider> = Arc::from(select_embedding_provider(None));
    let chain = Arc::new(ChainManager::new(0, 1000));
    let tier = SessionTier::new(embedder, chain.clone(), None);
    let conv = "budget";

    // Pre-warm the embedder (as the daemon does at startup).
    tier.warm().await;

    // Populate a realistic session window.
    for i in 0..64 {
        append_and_index(
            &tier,
            &chain,
            conv,
            &format!("turn {i}: some representative conversation content for indexing"),
        )
        .await;
    }

    // Warm the index: the first graft after a batch of inserts pays the one-shot
    // HNSW graph build (not part of the per-turn steady state — in production the
    // index is built incrementally across turns). Discard this measurement.
    let _ = tier
        .graft_block(conv, "warm-up graft to build the session index")
        .await;

    // Measure the steady-state per-turn graft latency (embed + scoped search).
    let start = Instant::now();
    let _ = tier
        .graft_block(conv, "representative query for the hot path")
        .await;
    let elapsed = start.elapsed().as_millis();
    assert!(
        elapsed <= GRAFT_HOT_PATH_BUDGET_MS,
        "graft hot-path latency {elapsed}ms exceeded budget {GRAFT_HOT_PATH_BUDGET_MS}ms"
    );
}
