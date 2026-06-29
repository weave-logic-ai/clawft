//! Session-scoped context graft layer (ADR-058 Phase 3).
//!
//! The agent loop never gets a parallel store; it gets a **query/graft layer**
//! over ExoChain's ECC indexes, scoped to one conversation. This module is the
//! kernel side of that layer:
//!
//! - **3.1 [`SessionView`]** — a session-scoped, *ephemeral* semantic index
//!   (a per-session HNSW). Because the index holds only this conversation's
//!   chunks, scoping is structural: a query can only return this session's
//!   chunks. The view is disposable and **rebuildable from the chain**
//!   (re-embed this session's events), so killing/rebuilding a loop costs only
//!   time — ExoChain remains the source of truth.
//! - **3.2 graft API** — query → candidate branches → COW-reference graft,
//!   keyed by chain sequence, large payloads as content-addressed blobs.
//! - **3.3 index population** — embed `agent.chat.turn` / tool-output events
//!   with the ADR-059 embedder, store f32 vectors keyed by chain sequence.
//!
//! ## This IS the ECC causal model
//!
//! Graft / prune / promote map onto the node lifecycle ([`NodeState`]):
//! `Speculative → Frontier → Committed` (collapse to trunk) or `→ Stale →
//! Pruned` (evicted off `main_line`). v1 is **single semantic index** per
//! conversation (ADR-058 phasing); fusion (causal + temporal + BVH) is v2 and
//! additive — every chunk is already keyed by chain sequence.
//!
//! The session view is deliberately separate from the durable global
//! [`CausalGraph`](crate::causal): it is the L2 "warm, this-run" tier. L2→L3
//! promotion onto the durable trunk is Phase 4.

use std::collections::HashSet;

use dashmap::DashMap;
use serde_json::json;

use crate::hnsw_service::{HnswService, HnswServiceConfig};

/// Node lifecycle state on the causal model (ADR-058 / ADR-056).
///
/// A grafted branch that coheres **collapses (commits)** to the trunk; one that
/// does not is held off `main_line` and pruned — the same branch/merge/prune
/// semantics as git ("causal collapse modeled on the git tree").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    /// A candidate branch not yet grafted into the working set.
    Speculative,
    /// Grafted and live in the working window this run.
    Frontier,
    /// Promoted (collapsed) onto the durable trunk (L3).
    Committed,
    /// Aged out of the live window; origin retained on the chain, re-graftable.
    Stale,
    /// Contradicted/revised; dropped off `main_line`.
    Pruned,
}

impl NodeState {
    /// Short stable tag for metadata/serialization.
    pub fn as_str(self) -> &'static str {
        match self {
            NodeState::Speculative => "speculative",
            NodeState::Frontier => "frontier",
            NodeState::Committed => "committed",
            NodeState::Stale => "stale",
            NodeState::Pruned => "pruned",
        }
    }
}

/// Metadata for one chunk indexed in a [`SessionView`].
///
/// Keyed by `chain_seq` (the universal ExoChain key). `content_hash` is the
/// content-addressed identity used for dedup and provenance; large chunk text
/// is held as a `blob_hash` (RVF content-addressed blob), small text inline.
#[derive(Debug, Clone)]
pub struct ChunkMeta {
    /// ExoChain sequence this chunk originates from.
    pub chain_seq: u64,
    /// BLAKE3 hex of the chunk text — content-addressed identity.
    pub content_hash: String,
    /// Causal-model state of this chunk.
    pub state: NodeState,
    /// Originating event kind, e.g. `"agent.chat.turn"` / `"tool.output"`.
    pub kind: String,
    /// Content-addressed blob hash when the text was externalized (large).
    pub blob_hash: Option<String>,
    /// Inline chunk text when small enough to keep in the manifest.
    pub inline: Option<String>,
}

/// A hit from a scoped query: the matched chunk plus its similarity score.
#[derive(Debug, Clone)]
pub struct ScopedHit {
    /// ExoChain sequence of the matched chunk.
    pub chain_seq: u64,
    /// Cosine similarity (higher is closer).
    pub score: f32,
    /// Full chunk metadata (provenance + content reference).
    pub meta: ChunkMeta,
}

/// How a grafted item's content is carried into the working set.
///
/// The graft is **by COW reference** — the origin chain entry is never removed.
/// Small text rides inline; large text is referenced by its content-addressed
/// blob hash (load via [`ArtifactStore`](crate::artifact_store::ArtifactStore));
/// a chunk whose text was not retained in this view is a bare reference to be
/// recovered from the chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraftContent {
    /// Inline chunk text (small).
    Inline(String),
    /// RVF content-addressed blob hash (large); load via the artifact store.
    Blob(String),
    /// Indexed but text not retained here; recover from the origin chain.
    Reference,
}

/// One item grafted into the working set (ADR-058 Phase 3.2).
///
/// Keyed by chain sequence and carrying a verifiable provenance backref
/// (`chain_seq` + `content_hash`): the agent can produce the witness chain for
/// any grafted context.
#[derive(Debug, Clone)]
pub struct GraftedItem {
    /// ExoChain sequence — the universal key / provenance backref.
    pub chain_seq: u64,
    /// BLAKE3 content identity — dedup key + provenance.
    pub content_hash: String,
    /// Originating event kind.
    pub kind: String,
    /// Causal-model state of the grafted node.
    pub state: NodeState,
    /// Similarity score that selected this item (higher is closer).
    pub score: f32,
    /// COW reference to the item's content.
    pub content: GraftContent,
}

/// Compute the content-addressed identity of a chunk's text.
pub fn content_hash(text: &str) -> String {
    blake3::hash(text.as_bytes()).to_hex().to_string()
}

/// Resolve a chunk's content into a COW reference (blob hash > inline > bare ref).
fn graft_content(meta: &ChunkMeta) -> GraftContent {
    if let Some(ref h) = meta.blob_hash {
        GraftContent::Blob(h.clone())
    } else if let Some(ref t) = meta.inline {
        GraftContent::Inline(t.clone())
    } else {
        GraftContent::Reference
    }
}

/// A session-scoped, ephemeral semantic view over this conversation's chain
/// chunks (ADR-058 Phase 3.1).
///
/// Holds a per-session HNSW (so queries are structurally scoped) plus a
/// manifest of `chain_seq → [`ChunkMeta`]`. Disposable: drop it at session end;
/// rebuild it from the chain by re-inserting this session's events.
pub struct SessionView {
    session_id: String,
    dims: usize,
    index: HnswService,
    /// chain_seq → chunk metadata (this session's manifest).
    chunks: DashMap<u64, ChunkMeta>,
}

impl SessionView {
    /// Create an empty session view whose ephemeral index expects `dims`-wide
    /// vectors (use the embedder's [`dimensions`](crate::embedding::EmbeddingProvider::dimensions);
    /// ADR-059 Qwen3 = 512, Mock fallback = its own dim).
    pub fn new(session_id: impl Into<String>, dims: usize) -> Self {
        let config = HnswServiceConfig {
            ef_search: 100,
            ef_construction: 200,
            default_dimensions: dims,
        };
        Self {
            session_id: session_id.into(),
            dims,
            index: HnswService::new(config),
            chunks: DashMap::new(),
        }
    }

    /// The conversation/session id this view is scoped to.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Vector dimensionality of the ephemeral index.
    pub fn dims(&self) -> usize {
        self.dims
    }

    /// Number of chunks indexed in this session view.
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// `true` when no chunks have been indexed yet.
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// The chain sequences indexed in this view (the rebuild manifest).
    pub fn chain_seqs(&self) -> Vec<u64> {
        let mut seqs: Vec<u64> = self.chunks.iter().map(|e| *e.key()).collect();
        seqs.sort_unstable();
        seqs
    }

    /// Fetch a chunk's metadata by chain sequence.
    pub fn chunk(&self, chain_seq: u64) -> Option<ChunkMeta> {
        self.chunks.get(&chain_seq).map(|e| e.value().clone())
    }

    /// Insert a pre-embedded chunk into the ephemeral index, keyed by chain
    /// sequence. Low-level building block: 3.3 wraps this with the embedder and
    /// blob externalization. Replaces any existing chunk for the same
    /// `chain_seq` (idempotent re-graft / rebuild).
    ///
    /// Returns `false` (and does not index) when `vector` does not match the
    /// view's dimensionality — callers must embed with the configured model.
    pub fn insert_vector(&self, vector: Vec<f32>, meta: ChunkMeta) -> bool {
        if vector.len() != self.dims {
            return false;
        }
        let chain_seq = meta.chain_seq;
        let metadata = json!({
            "chain_seq": chain_seq,
            "content_hash": meta.content_hash,
            "state": meta.state.as_str(),
            "kind": meta.kind,
            "session_id": self.session_id,
        });
        self.index.insert(chain_seq.to_string(), vector, metadata);
        self.chunks.insert(chain_seq, meta);
        true
    }

    /// Query the session-scoped index for the `top_k` nearest chunks.
    ///
    /// Because the index holds only this session's chunks, results are
    /// structurally scoped — a hit can only be from this conversation.
    pub fn query(&self, query: &[f32], top_k: usize) -> Vec<ScopedHit> {
        if query.len() != self.dims {
            return Vec::new();
        }
        self.index
            .search(query, top_k)
            .into_iter()
            .filter_map(|hit| {
                let chain_seq: u64 = hit.id.parse().ok()?;
                let meta = self.chunk(chain_seq)?;
                Some(ScopedHit {
                    chain_seq,
                    score: hit.score,
                    meta,
                })
            })
            .collect()
    }

    /// Graft the `top_k` most relevant chunks into the working set (ADR-058
    /// Phase 3.2).
    ///
    /// Scoped query → candidate branches → **COW-reference** graft: returns
    /// chain-sequence-keyed [`GraftedItem`]s with provenance backrefs
    /// (`chain_seq` + `content_hash`) and a content reference (inline / blob /
    /// bare). Candidates are **deduplicated by content hash** — identical tool
    /// outputs or repeated file reads collapse to one item (the highest-scoring
    /// occurrence). The origin is never removed; this only references it.
    pub fn graft(&self, query: &[f32], top_k: usize) -> Vec<GraftedItem> {
        if top_k == 0 {
            return Vec::new();
        }
        // Over-fetch so dedup-by-content does not starve the result below top_k.
        let raw = self.query(query, top_k.saturating_mul(2));
        let mut seen: HashSet<String> = HashSet::new();
        let mut out: Vec<GraftedItem> = Vec::with_capacity(top_k);
        // `query` returns hits sorted by descending score, so the first time a
        // content hash is seen is its best-scoring occurrence.
        for hit in raw {
            if !seen.insert(hit.meta.content_hash.clone()) {
                continue; // duplicate content — already grafted at a higher score
            }
            out.push(GraftedItem {
                chain_seq: hit.chain_seq,
                content_hash: hit.meta.content_hash.clone(),
                kind: hit.meta.kind.clone(),
                state: hit.meta.state,
                score: hit.score,
                content: graft_content(&hit.meta),
            });
            if out.len() == top_k {
                break;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
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
}
