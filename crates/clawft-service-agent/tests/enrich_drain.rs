//! Phase B (classification-design §D4, plan B3) — the async enrichment
//! refinement path, end-to-end minus the tokio drain plumbing.
//!
//! The daemon's `run_enrich_drain` does exactly this per job: read the node's
//! keyword topic, run the cheap-model [`EnrichmentClassifier`], then patch the
//! `classification` blob via [`CausalGraph::merge_node_metadata`] (B1). These
//! tests drive that same sequence against a mock backend to prove the two B3
//! contracts on the refinement side — the blob flips `keyword → llm` and is
//! visible on the next graph read, and a re-patch is idempotent — plus the
//! queue's drop-oldest/non-blocking overflow that keeps `index_turn` free.

use async_trait::async_trait;
use serde_json::json;

use clawft_core::agent::context_router::Classifier;
use clawft_kernel::causal::CausalGraph;
use clawft_service_agent::{EnrichJob, EnrichQueue, EnrichmentClassifier};

/// Canned LLM backend returning a fixed four-axis body (design §D2 shape).
struct MockBackend(Result<String, String>);

#[async_trait]
impl Classifier for MockBackend {
    async fn classify(
        &self,
        _user_content: &str,
        _max_tokens: u32,
        _model_override: Option<&str>,
    ) -> Result<String, String> {
        self.0.clone()
    }
}

const LLM_BODY: &str = r#"{"intent":"correction","topic":"build",
  "emotion":{"valence":-0.6,"arousal":0.8,"dominance":0.2,"label":"frustrated"},
  "goal":"fix the build"}"#;

/// Add a committed turn node carrying the Phase-A keyword blob plus the
/// protected identity/lifecycle keys, mirroring a real dual-written turn.
fn add_keyword_turn(g: &CausalGraph, conv_id: &str) -> u64 {
    g.add_node(
        "turn".into(),
        json!({
            "conv_id": conv_id,
            "state": "committed",
            "uid": "turn-uid-1",
            "chain_seq": 7,
            "text": "no, that broke the build!",
            "classification": {
                "intent": "statement",
                "topic": "build",
                "emotion": { "valence": 0.0, "arousal": 0.5, "dominance": 0.0, "label": "neutral" },
                "goal": null,
                "tier": "keyword",
                "v": 1
            }
        }),
    )
}

/// The exact per-job refinement `run_enrich_drain` performs, factored so the
/// test drives the real code paths (enrich + merge) without the daemon.
async fn refine_one(g: &CausalGraph, enricher: &EnrichmentClassifier, job: &EnrichJob) -> bool {
    let prev_topic = g.get_node(job.node_id).and_then(|n| {
        n.metadata
            .get("classification")
            .and_then(|c| c.get("topic"))
            .and_then(|t| t.as_str())
            .map(str::to_string)
    });
    let Some(cv) = enricher.enrich(&job.text, prev_topic.as_deref()).await else {
        return false;
    };
    let mut patch = serde_json::Map::new();
    patch.insert("classification".to_string(), cv.to_metadata_value());
    g.merge_node_metadata(job.node_id, &patch)
}

#[tokio::test]
async fn keyword_patch_flips_to_llm_and_reflects_on_next_read() {
    let g = CausalGraph::new();
    let id = add_keyword_turn(&g, "conv-1");
    let enricher = EnrichmentClassifier::from_backend(std::sync::Arc::new(MockBackend(Ok(
        LLM_BODY.to_string(),
    ))));

    let job = EnrichJob {
        conv_id: "conv-1".into(),
        node_id: id,
        chain_seq: 7,
        text: "no, that broke the build!".into(),
    };
    assert!(refine_one(&g, &enricher, &job).await);

    // The RPC reads node metadata verbatim via `nodes_for_conv` — assert the
    // refinement is visible there (design §D3 "reflects on next read").
    let node = g
        .nodes_for_conv("conv-1")
        .into_iter()
        .find(|n| n.id == id)
        .expect("turn node in conversation projection");
    let cls = &node.metadata["classification"];
    assert_eq!(cls["tier"], "llm", "keyword blob flipped to llm tier");
    assert_eq!(cls["intent"], "correction", "intent refined by the LLM");
    assert_eq!(cls["goal"], "fix the build", "goal filled by the LLM");
    assert!((cls["emotion"]["arousal"].as_f64().unwrap() - 0.8).abs() < 1e-6);

    // Protected identity/lifecycle keys survive the patch (B1).
    assert_eq!(node.metadata["state"], "committed");
    assert_eq!(node.metadata["uid"], "turn-uid-1");
    assert_eq!(node.metadata["chain_seq"], 7);
    assert_eq!(node.metadata["text"], "no, that broke the build!");
    assert_eq!(node.metadata["conv_id"], "conv-1");
}

#[tokio::test]
async fn re_patch_is_idempotent() {
    let g = CausalGraph::new();
    let id = add_keyword_turn(&g, "conv-1");
    let enricher = EnrichmentClassifier::from_backend(std::sync::Arc::new(MockBackend(Ok(
        LLM_BODY.to_string(),
    ))));
    let job = EnrichJob {
        conv_id: "conv-1".into(),
        node_id: id,
        chain_seq: 7,
        text: "no, that broke the build!".into(),
    };

    assert!(refine_one(&g, &enricher, &job).await);
    let after_first = g.get_node(id).unwrap().metadata["classification"].clone();
    assert!(refine_one(&g, &enricher, &job).await);
    let after_second = g.get_node(id).unwrap().metadata["classification"].clone();

    assert_eq!(
        after_first, after_second,
        "re-patching the same refinement is a no-op on the blob"
    );
    assert_eq!(after_second["tier"], "llm");
}

#[tokio::test]
async fn backend_failure_keeps_keyword_blob() {
    let g = CausalGraph::new();
    let id = add_keyword_turn(&g, "conv-1");
    // Backend errors → enrich returns None → node keeps its durable keyword blob.
    let enricher = EnrichmentClassifier::from_backend(std::sync::Arc::new(MockBackend(Err(
        "backend down".into(),
    ))));
    let job = EnrichJob {
        conv_id: "conv-1".into(),
        node_id: id,
        chain_seq: 7,
        text: "no, that broke the build!".into(),
    };
    assert!(!refine_one(&g, &enricher, &job).await, "no patch applied");
    assert_eq!(
        g.get_node(id).unwrap().metadata["classification"]["tier"],
        "keyword",
        "keyword blob preserved on backend failure"
    );
}

#[test]
fn queue_full_drops_oldest_without_blocking_the_turn() {
    // The enqueue seam `index_turn` uses: bound 3, five turns committed in a
    // burst. Enqueue is synchronous and must never block; the two oldest jobs
    // are shed so the queue never exceeds its bound.
    let q = EnrichQueue::new(3);
    for seq in 1..=5u64 {
        q.enqueue(EnrichJob {
            conv_id: "conv-1".into(),
            node_id: seq,
            chain_seq: seq,
            text: format!("turn {seq}"),
        });
    }
    assert_eq!(q.len(), 3, "queue capped at its bound, oldest dropped");

    // Survivors are the three newest (3,4,5), still oldest-first.
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    rt.block_on(async {
        assert_eq!(q.recv().await.node_id, 3);
        assert_eq!(q.recv().await.node_id, 4);
        assert_eq!(q.recv().await.node_id, 5);
    });
}
