//! Unit tests for `context_graft` (ADR-058 Phase 3). Split to a sibling
//! file for the <500-line rule. No live model required (Mock embedder).

use super::*;

/// Build a deterministic unit-ish vector pointing mostly along axis `axis`.
fn axis_vec(dims: usize, axis: usize) -> Vec<f32> {
    let mut v = vec![0.01_f32; dims];
    if axis < dims {
        v[axis] = 1.0;
    }
    v
}

fn meta(chain_seq: u64, text: &str) -> ChunkMeta {
    ChunkMeta {
        chain_seq,
        content_hash: content_hash(text),
        state: NodeState::Frontier,
        kind: "agent.chat.turn".into(),
        blob_hash: None,
        inline: Some(text.into()),
    }
}

#[test]
fn node_state_tags_are_stable() {
    assert_eq!(NodeState::Speculative.as_str(), "speculative");
    assert_eq!(NodeState::Committed.as_str(), "committed");
    assert_eq!(NodeState::Pruned.as_str(), "pruned");
}

#[test]
fn insert_rejects_dim_mismatch() {
    let view = SessionView::new("s1", 8);
    assert!(!view.insert_vector(vec![1.0; 4], meta(1, "x")));
    assert!(view.is_empty());
}

#[test]
fn query_returns_indexed_chunk() {
    let view = SessionView::new("s1", 8);
    assert!(view.insert_vector(axis_vec(8, 0), meta(10, "alpha")));
    assert!(view.insert_vector(axis_vec(8, 3), meta(11, "beta")));
    assert_eq!(view.len(), 2);

    let hits = view.query(&axis_vec(8, 0), 1);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].chain_seq, 10);
    assert_eq!(hits[0].meta.inline.as_deref(), Some("alpha"));
}

/// Two independent sessions: each query returns ONLY its own session's
/// chunks (structural scoping — Done-when 3.1).
#[test]
fn query_is_scoped_to_session() {
    let s1 = SessionView::new("s1", 8);
    let s2 = SessionView::new("s2", 8);
    s1.insert_vector(axis_vec(8, 0), meta(100, "s1-chunk"));
    s2.insert_vector(axis_vec(8, 0), meta(200, "s2-chunk"));

    let h1 = s1.query(&axis_vec(8, 0), 5);
    let h2 = s2.query(&axis_vec(8, 0), 5);
    assert_eq!(h1.len(), 1);
    assert_eq!(h1[0].chain_seq, 100);
    assert_eq!(h2.len(), 1);
    assert_eq!(h2[0].chain_seq, 200);
}

/// The view is rebuildable from the chain: re-inserting the same seqs/vectors
/// reproduces the same manifest and query result.
#[test]
fn view_is_rebuildable() {
    let build = || {
        let v = SessionView::new("s1", 8);
        v.insert_vector(axis_vec(8, 0), meta(1, "a"));
        v.insert_vector(axis_vec(8, 1), meta(2, "b"));
        v.insert_vector(axis_vec(8, 2), meta(3, "c"));
        v
    };
    let a = build();
    let b = build();
    assert_eq!(a.chain_seqs(), b.chain_seqs());
    assert_eq!(a.chain_seqs(), vec![1, 2, 3]);

    let qa = a.query(&axis_vec(8, 1), 1);
    let qb = b.query(&axis_vec(8, 1), 1);
    assert_eq!(qa[0].chain_seq, qb[0].chain_seq);
    assert_eq!(qa[0].chain_seq, 2);
}

/// Re-inserting the same chain_seq replaces (idempotent re-graft).
#[test]
fn reinsert_same_seq_is_idempotent() {
    let view = SessionView::new("s1", 8);
    view.insert_vector(axis_vec(8, 0), meta(7, "first"));
    view.insert_vector(axis_vec(8, 0), meta(7, "second"));
    assert_eq!(view.len(), 1);
    assert_eq!(view.chunk(7).unwrap().inline.as_deref(), Some("second"));
}

// ── 3.2 graft API ────────────────────────────────────────────────

#[test]
fn graft_returns_chain_seq_keyed_items_with_provenance() {
    let view = SessionView::new("s1", 8);
    view.insert_vector(axis_vec(8, 0), meta(42, "alpha"));
    let items = view.graft(&axis_vec(8, 0), 1);
    assert_eq!(items.len(), 1);
    let it = &items[0];
    assert_eq!(it.chain_seq, 42); // chain-sequence keyed
    assert_eq!(it.content_hash, content_hash("alpha")); // provenance backref
    assert_eq!(it.kind, "agent.chat.turn");
    assert_eq!(it.content, GraftContent::Inline("alpha".into()));
}

/// Identical content at two chain sequences dedups to a single graft.
#[test]
fn graft_dedups_by_content_hash() {
    let view = SessionView::new("s1", 8);
    // Same text "dup" (same content_hash), identical vectors, two seqs.
    view.insert_vector(axis_vec(8, 0), meta(1, "dup"));
    view.insert_vector(axis_vec(8, 0), meta(2, "dup"));
    // A distinct chunk so the index is non-trivial.
    view.insert_vector(axis_vec(8, 5), meta(3, "other"));
    assert_eq!(view.len(), 3);

    let items = view.graft(&axis_vec(8, 0), 5);
    let dup_hits: Vec<_> = items
        .iter()
        .filter(|i| i.content_hash == content_hash("dup"))
        .collect();
    assert_eq!(dup_hits.len(), 1, "duplicate content must collapse to one");
    assert!(dup_hits[0].chain_seq == 1 || dup_hits[0].chain_seq == 2);
}

#[test]
fn graft_externalized_chunk_is_blob_ref() {
    let view = SessionView::new("s1", 8);
    let m = ChunkMeta {
        chain_seq: 9,
        content_hash: content_hash("huge payload"),
        state: NodeState::Frontier,
        kind: "tool.output".into(),
        blob_hash: Some("blake3hash".into()),
        inline: None,
    };
    view.insert_vector(axis_vec(8, 0), m);
    let items = view.graft(&axis_vec(8, 0), 1);
    assert_eq!(items[0].content, GraftContent::Blob("blake3hash".into()));
}

#[test]
fn graft_top_k_zero_is_empty() {
    let view = SessionView::new("s1", 8);
    view.insert_vector(axis_vec(8, 0), meta(1, "a"));
    assert!(view.graft(&axis_vec(8, 0), 0).is_empty());
}

// ── 3.3 index population (embedder + content-addressed blobs) ─────

use crate::embedding::MockEmbeddingProvider;

/// Embedded events are indexed and retrievable (Done-when 3.3). The Mock
/// embedder is deterministic, so embedding the same text for index and query
/// yields the same vector and the chunk is recalled.
#[tokio::test]
async fn index_chunk_then_graft_retrieves() {
    let embedder = MockEmbeddingProvider::new(32);
    let view = SessionView::for_embedder("s1", &embedder);
    assert_eq!(view.dims(), 32);

    view.index_chunk(
        &embedder,
        None,
        1,
        "agent.chat.turn",
        "the sky is blue",
        4096,
    )
    .await
    .unwrap();
    view.index_chunk(&embedder, None, 2, "agent.chat.turn", "rust is fast", 4096)
        .await
        .unwrap();
    assert_eq!(view.len(), 2);

    let hits = view
        .graft_text(&embedder, "the sky is blue", 1)
        .await
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].chain_seq, 1);
    assert_eq!(
        hits[0].content,
        GraftContent::Inline("the sky is blue".into())
    );
}

/// A large payload is externalized to a content-addressed blob; the chunk
/// holds only the hash, and the blob is loadable from the artifact store.
#[tokio::test]
async fn index_large_chunk_externalizes_to_blob() {
    let embedder = MockEmbeddingProvider::new(32);
    let store = ArtifactStore::new_memory();
    let view = SessionView::for_embedder("s1", &embedder);

    let big = "X".repeat(500);
    view.index_chunk(&embedder, Some(&store), 7, "tool.output", &big, 64)
        .await
        .unwrap();

    let chunk = view.chunk(7).unwrap();
    assert!(chunk.inline.is_none(), "large chunk must not be inline");
    let blob_hash = chunk.blob_hash.expect("large chunk should be a blob");
    // The blob is content-addressed and loadable.
    let loaded = store.load(&blob_hash).unwrap();
    assert_eq!(loaded, big.as_bytes());

    // Still retrievable via the index even though the text is externalized.
    let q = embedder.embed(&big).await.unwrap();
    let items = view.graft(&q, 1);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].chain_seq, 7);
    assert!(matches!(items[0].content, GraftContent::Blob(_)));
}

/// A small payload stays inline (no blob written).
#[tokio::test]
async fn index_small_chunk_stays_inline() {
    let embedder = MockEmbeddingProvider::new(32);
    let store = ArtifactStore::new_memory();
    let view = SessionView::for_embedder("s1", &embedder);

    view.index_chunk(&embedder, Some(&store), 3, "agent.chat.turn", "short", 64)
        .await
        .unwrap();
    let chunk = view.chunk(3).unwrap();
    assert_eq!(chunk.inline.as_deref(), Some("short"));
    assert!(chunk.blob_hash.is_none());
}

// ── 4.1 prune-to-graft eviction ──────────────────────────────────────

/// A pruned chunk leaves the live window (→ Stale) but stays in the index and
/// is re-grafted correctly later; regraft restores it to Frontier (Done-when
/// 4.1).
#[test]
fn prune_keeps_chunk_regraftable() {
    let view = SessionView::new("s1", 8);
    view.insert_vector(axis_vec(8, 0), meta(10, "fact"));
    assert_eq!(view.live_seqs(), vec![10]);

    // Prune: evicted from the live window, origin retained in the index.
    assert!(view.prune(10));
    assert_eq!(view.chunk(10).unwrap().state, NodeState::Stale);
    assert!(view.live_seqs().is_empty());

    // Re-graftable: a query still recalls it.
    let items = view.graft(&axis_vec(8, 0), 1);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].chain_seq, 10);
    assert_eq!(items[0].state, NodeState::Stale);

    // Re-graft restores it to the live window.
    assert!(view.regraft(10));
    assert_eq!(view.chunk(10).unwrap().state, NodeState::Frontier);
    assert_eq!(view.live_seqs(), vec![10]);
}

#[test]
fn prune_unknown_seq_is_false() {
    let view = SessionView::new("s1", 8);
    assert!(!view.prune(999));
    assert!(!view.regraft(999));
    assert!(!view.set_state(999, NodeState::Committed));
}
