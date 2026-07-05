//! Wave 1 §W1.2 wire round-trip: a `voice_analysis` blob handed to
//! [`SessionTier::index_turn`] is (a) stored verbatim as a sibling node
//! metadata key and (b) its `emotion` sub-blob overrides ONLY the compact
//! `classification` emotion axis with `tier:"voice"` (the voice > llm > keyword
//! confidence hierarchy), leaving intent/topic keyword-derived and the compact
//! axis's four-field shape intact. Device-free and deterministic (Mock embedder,
//! in-memory chain + causal graph) — the full mic→daemon path is the tester's
//! `assembly.rs` exit test; this pins the durable seam the record rides on.

use std::sync::Arc;

use clawft_kernel::chain::ChainManager;
use clawft_kernel::embedding::{EmbeddingProvider, MockEmbeddingProvider};
use clawft_kernel::{CausalGraph, CrossRefStore};
use clawft_service_agent::{KeywordTurnClassifier, SessionTier};
use serde_json::json;

const DIMS: usize = 64;

/// A forest-joined tier with the keyword classifier attached (the Step-0
/// `mode = "keyword"` daemon shape), plus a handle to the causal graph so the
/// test can read the committed node's metadata back.
fn forest_tier_with_classifier() -> (SessionTier, Arc<ChainManager>, Arc<CausalGraph>) {
    let embedder: Arc<dyn EmbeddingProvider> = Arc::new(MockEmbeddingProvider::new(DIMS));
    let chain = Arc::new(ChainManager::new(0, 1000));
    let causal = Arc::new(CausalGraph::new());
    let crossrefs = Arc::new(CrossRefStore::new());
    let tier = SessionTier::new(embedder, chain.clone(), None)
        .with_forest(causal.clone(), crossrefs.clone())
        .with_classifier(Arc::new(KeywordTurnClassifier::new()));
    (tier, chain, causal)
}

/// The §W1.2 record shape (representative subset): a high-arousal acted turn.
fn acted_voice_analysis() -> serde_json::Value {
    json!({
        "v": 1,
        "tier": "voice",
        "stt": {
            "model": "parakeet-tdt-0.6b", "path": "native", "latency_ms": 48,
            "tokens": [{ "text": "stop", "t_ms": 120, "dur_ms": 240, "conf": 0.97 }],
            "token_conf_mean": 0.97, "token_conf_min": 0.97
        },
        "audio": {
            "duration_ms": 900, "voiced_ms": 700, "silence_ms": 200,
            "rms_dbfs_mean": -18.0, "rms_dbfs_peak": -6.0,
            "noise_floor_dbfs": -52.0, "snr_db": 34.0, "clip_pct": 0.0, "dc_offset": 2
        },
        "prosody": {
            "f0_mean_hz": 210.0, "f0_min_hz": 140.0, "f0_max_hz": 330.0,
            "f0_range_semitones": 14.8, "f0_slope": 1.2,
            "rate_tokens_per_s": 5.1, "pause_count": 0, "pause_mean_ms": 0.0,
            "energy_dynamics_db": 22.0, "confidence": "medium"
        },
        "emotion": {
            "valence": -0.45, "arousal": 0.92, "dominance": 0.30, "label": "agitated",
            "arousal_conf": "medium", "valence_conf": "low", "source": "prosody-dsp"
        },
        "paralinguistics": { "non_lexical": false, "class": "speech", "confidence": "low" }
    })
}

#[tokio::test]
async fn voice_analysis_stored_verbatim_and_emotion_axis_flips_to_voice() {
    let (tier, chain, causal) = forest_tier_with_classifier();
    let conv = "voice-wire-cv";
    let va = acted_voice_analysis();

    // Mirror the substrate sink: append to the witness chain, then index the
    // turn with the voice record attached (as `KernelTurnAnchor` now does).
    let ev = chain.append(
        "agent",
        "agent.chat.turn",
        Some(json!({ "conv": conv, "content": "this is completely fine" })),
    );
    tier.index_turn(
        conv,
        ev.sequence,
        "agent.chat.turn",
        "user",
        // Deliberately calm TEXT so the keyword emotion is neutral — proving the
        // flip comes from the voice record, not the lexical classifier.
        "this is completely fine",
        Some(&va),
    )
    .await;

    let nodes = causal.nodes_for_conv(conv);
    assert_eq!(nodes.len(), 1, "exactly one turn node committed");
    let meta = &nodes[0].metadata;

    // (a) The full record rides verbatim on the sibling key.
    let stored = meta
        .get("voice_analysis")
        .expect("sibling voice_analysis key present on the turn node");
    assert_eq!(stored, &va, "voice_analysis stored verbatim (flat-key parity)");
    // Spot-check nested fields survive round-trip unreshaped.
    assert_eq!(stored["audio"]["snr_db"], json!(34.0));
    assert_eq!(stored["prosody"]["f0_mean_hz"], json!(210.0));
    assert_eq!(stored["stt"]["tokens"][0]["conf"], json!(0.97));
    assert_eq!(stored["emotion"]["source"], json!("prosody-dsp"));

    // (b) The compact classification emotion axis is overridden with the voice
    // values and the blob's provenance tier flips to "voice".
    let cls = meta
        .get("classification")
        .expect("classification blob present (mode=keyword)");
    assert_eq!(cls["tier"], json!("voice"), "emotion provenance flips to voice");
    assert_eq!(cls["emotion"]["valence"], json!(-0.45));
    assert_eq!(cls["emotion"]["arousal"], json!(0.92));
    assert_eq!(cls["emotion"]["dominance"], json!(0.30));
    assert_eq!(cls["emotion"]["label"], json!("agitated"));

    // Intent/topic remain keyword-derived (only the emotion axis is authoritative
    // from voice). "this is completely fine" is a declarative statement.
    assert_eq!(cls["intent"], json!("statement"));
    assert!(cls.get("topic").is_some(), "topic still populated by keyword tier");

    // The compact axis keeps its four-field VAD shape — the rich confidence
    // flags / source stay in the sibling record, never leak into the small
    // cross-modality contract.
    assert!(
        cls["emotion"].get("arousal_conf").is_none()
            && cls["emotion"].get("source").is_none(),
        "compact emotion axis must not absorb the sibling's rich fields"
    );

    // The turn's text is stored at rest alongside the classification (design §6).
    assert_eq!(meta.get("text").and_then(|v| v.as_str()), Some("this is completely fine"));
}

#[tokio::test]
async fn no_voice_analysis_leaves_classification_keyword_and_no_sibling_key() {
    // Regression guard: a plain text turn (voice_analysis = None) is untouched —
    // no sibling key, emotion axis stays keyword. Proves the merge is opt-in and
    // text turns carry zero voice fields.
    let (tier, chain, causal) = forest_tier_with_classifier();
    let conv = "text-only-cv";

    let ev = chain.append(
        "agent",
        "agent.chat.turn",
        Some(json!({ "conv": conv, "content": "what is the launch code" })),
    );
    tier.index_turn(conv, ev.sequence, "agent.chat.turn", "user", "what is the launch code", None)
        .await;

    let nodes = causal.nodes_for_conv(conv);
    assert_eq!(nodes.len(), 1);
    let meta = &nodes[0].metadata;

    assert!(
        meta.get("voice_analysis").is_none(),
        "text turn carries no voice_analysis sibling key"
    );
    let cls = meta.get("classification").expect("keyword classification present");
    assert_eq!(cls["tier"], json!("keyword"), "text turn stays keyword-tier");
}
