//! Edge embeddings for relationship-level retrieval (WEFT-375 / LightRAG P5).
//!
//! Embeds relationships (not just entities) so queries like
//! "how does X interact with Y?" retrieve relevant edges directly.
//!
//! # Design
//!
//! - Each [`Relationship`] may carry an optional `embedding` vector.
//! - [`EdgeEmbeddingIndex`] builds a searchable in-memory index over edges
//!   using either pre-populated embeddings or the same char-trigram local
//!   embedder used by entity alignment ([`crate::alignment::local_label_embedding`]).
//! - [`query_relationships`] is the relationship-anchored query path.
//! - With `kernel-bridge`, edges are also inserted into `HnswService` under
//!   ids prefixed with [`EDGE_HNSW_PREFIX`] (`rel::`).

use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};

use crate::alignment::{cosine_similarity, local_label_embedding};
use crate::entity::EntityId;
use crate::model::KnowledgeGraph;
use crate::relationship::{RelationType, Relationship};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default dimensionality for local edge embeddings (matches alignment).
pub const DEFAULT_EDGE_EMBED_DIMS: usize = 64;

/// HNSW id prefix for relationship (edge) embeddings.
///
/// Full id form: `rel::{stable_id}` where `stable_id` is
/// [`Relationship::stable_id`].
pub const EDGE_HNSW_PREFIX: &str = "rel::";

// ---------------------------------------------------------------------------
// Search result
// ---------------------------------------------------------------------------

/// A single hit from relationship-anchored retrieval.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EdgeSearchResult {
    /// Stable relationship id ([`Relationship::stable_id`]).
    pub relationship_id: String,
    pub source: EntityId,
    pub target: EntityId,
    /// Snake_case relation type string.
    pub relation_type: String,
    /// Cosine similarity in `[0, 1]` (mapped from `[-1, 1]`).
    pub score: f64,
    pub source_label: String,
    pub target_label: String,
    /// Text that was (or would be) embedded for this edge.
    pub embed_text: String,
}

// ---------------------------------------------------------------------------
// Index entry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct EdgeEntry {
    relationship_id: String,
    source: EntityId,
    target: EntityId,
    relation_type: String,
    source_label: String,
    target_label: String,
    embed_text: String,
    embedding: Vec<f32>,
}

// ---------------------------------------------------------------------------
// EdgeEmbeddingIndex
// ---------------------------------------------------------------------------

/// In-memory index of relationship embeddings for edge-level queries.
///
/// Built from a [`KnowledgeGraph`]. Prefer pre-populated
/// [`Relationship::embedding`] when present; otherwise compute a local
/// char-trigram embedding from the edge's semantic text.
#[derive(Debug, Clone)]
pub struct EdgeEmbeddingIndex {
    entries: Vec<EdgeEntry>,
    dims: usize,
}

impl EdgeEmbeddingIndex {
    /// Build an index using local char-trigram embeddings at `dims`.
    ///
    /// Existing [`Relationship::embedding`] vectors are reused when their
    /// length matches `dims`; otherwise they are recomputed from labels.
    pub fn build(kg: &KnowledgeGraph, dims: usize) -> Self {
        let dims = dims.max(8);
        let mut entries = Vec::with_capacity(kg.relationship_count());

        for (src, tgt, rel) in kg.edges() {
            let embed_text = Relationship::embed_text_with_context(
                &src.label,
                &rel.relation_type,
                &tgt.label,
                rel.source_file.as_deref(),
            );
            let embedding = match rel.embedding.as_ref() {
                Some(v) if v.len() == dims => v.clone(),
                _ => local_label_embedding(&embed_text, dims),
            };
            entries.push(EdgeEntry {
                relationship_id: rel.stable_id(),
                source: src.id.clone(),
                target: tgt.id.clone(),
                relation_type: rel.relation_type_str(),
                source_label: src.label.clone(),
                target_label: tgt.label.clone(),
                embed_text,
                embedding,
            });
        }

        Self { entries, dims }
    }

    /// Build with a custom embedder (e.g. neural / provider-backed).
    ///
    /// When a relationship already has a non-empty embedding, that vector is
    /// kept; otherwise `embed_fn(embed_text)` is used.
    pub fn build_with_embedder<F>(kg: &KnowledgeGraph, mut embed_fn: F) -> Self
    where
        F: FnMut(&str) -> Vec<f32>,
    {
        let mut entries = Vec::with_capacity(kg.relationship_count());
        let mut dims = 0usize;

        for (src, tgt, rel) in kg.edges() {
            let embed_text = Relationship::embed_text_with_context(
                &src.label,
                &rel.relation_type,
                &tgt.label,
                rel.source_file.as_deref(),
            );
            let embedding = match rel.embedding.as_ref() {
                Some(v) if !v.is_empty() => v.clone(),
                _ => embed_fn(&embed_text),
            };
            if dims == 0 {
                dims = embedding.len().max(8);
            }
            entries.push(EdgeEntry {
                relationship_id: rel.stable_id(),
                source: src.id.clone(),
                target: tgt.id.clone(),
                relation_type: rel.relation_type_str(),
                source_label: src.label.clone(),
                target_label: tgt.label.clone(),
                embed_text,
                embedding,
            });
        }

        Self {
            entries,
            dims: dims.max(8),
        }
    }

    /// Number of indexed edges.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Embedding dimensionality used by this index.
    pub fn dims(&self) -> usize {
        self.dims
    }

    /// Search by free-text query (locally embedded with the same dims).
    pub fn search_text(&self, query: &str, top_k: usize) -> Vec<EdgeSearchResult> {
        if top_k == 0 || self.entries.is_empty() {
            return Vec::new();
        }
        let q = local_label_embedding(query, self.dims);
        self.search_vector(&q, top_k)
    }

    /// Search by pre-computed query vector.
    pub fn search_vector(&self, query: &[f32], top_k: usize) -> Vec<EdgeSearchResult> {
        if top_k == 0 || self.entries.is_empty() {
            return Vec::new();
        }

        let mut scored: Vec<(f64, usize)> = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| (cosine_similarity(query, &e.embedding), i))
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);

        scored
            .into_iter()
            .map(|(score, i)| {
                let e = &self.entries[i];
                EdgeSearchResult {
                    relationship_id: e.relationship_id.clone(),
                    source: e.source.clone(),
                    target: e.target.clone(),
                    relation_type: e.relation_type.clone(),
                    score,
                    source_label: e.source_label.clone(),
                    target_label: e.target_label.clone(),
                    embed_text: e.embed_text.clone(),
                }
            })
            .collect()
    }

    /// Iterate stored relationship ids (for HNSW sync / tests).
    pub fn relationship_ids(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|e| e.relationship_id.as_str())
    }

    /// HNSW entry id for an edge: `rel::{stable_id}`.
    pub fn hnsw_id_for(stable_id: &str) -> String {
        format!("{EDGE_HNSW_PREFIX}{stable_id}")
    }

    /// Snapshot entries as (hnsw_id, embedding, metadata) for external indexes.
    pub fn hnsw_entries(&self) -> Vec<(String, Vec<f32>, serde_json::Value)> {
        self.entries
            .iter()
            .map(|e| {
                let id = Self::hnsw_id_for(&e.relationship_id);
                let meta = serde_json::json!({
                    "kind": "relationship",
                    "relationship_id": e.relationship_id,
                    "source": e.source.to_hex(),
                    "target": e.target.to_hex(),
                    "relation_type": e.relation_type,
                    "source_label": e.source_label,
                    "target_label": e.target_label,
                    "embed_text": e.embed_text,
                });
                (id, e.embedding.clone(), meta)
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Populate + query helpers
// ---------------------------------------------------------------------------

/// Write local embeddings onto every relationship in the graph.
///
/// Fills [`Relationship::embedding`] so the field is present for export and
/// for consumers that read edges directly.
pub fn populate_edge_embeddings(kg: &mut KnowledgeGraph, dims: usize) {
    let dims = dims.max(8);
    // Collect (edge_index, embedding) then apply — petgraph allows edge_weights_mut.
    let plan: Vec<(petgraph::graph::EdgeIndex, Vec<f32>)> = {
        let graph = kg.inner_graph();
        graph
            .edge_references()
            .map(|e| {
                let src = &graph[e.source()];
                let tgt = &graph[e.target()];
                let rel = e.weight();
                let text = Relationship::embed_text_with_context(
                    &src.label,
                    &rel.relation_type,
                    &tgt.label,
                    rel.source_file.as_deref(),
                );
                let emb = match rel.embedding.as_ref() {
                    Some(v) if v.len() == dims => v.clone(),
                    _ => local_label_embedding(&text, dims),
                };
                (e.id(), emb)
            })
            .collect()
    };

    for (ei, emb) in plan {
        if let Some(rel) = kg.edge_weight_mut(ei) {
            rel.embedding = Some(emb);
        }
    }
}

/// Relationship-anchored retrieval: embed `query` and return top-k edges.
///
/// Convenience wrapper around [`EdgeEmbeddingIndex::build`] + `search_text`.
pub fn query_relationships(
    kg: &KnowledgeGraph,
    query: &str,
    top_k: usize,
) -> Vec<EdgeSearchResult> {
    query_relationships_dims(kg, query, top_k, DEFAULT_EDGE_EMBED_DIMS)
}

/// Same as [`query_relationships`] with an explicit embedding dimension.
pub fn query_relationships_dims(
    kg: &KnowledgeGraph,
    query: &str,
    top_k: usize,
    dims: usize,
) -> Vec<EdgeSearchResult> {
    let index = EdgeEmbeddingIndex::build(kg, dims);
    index.search_text(query, top_k)
}

/// Build multi-view embed texts for one edge (for multi-key HNSW if desired).
///
/// Returns `(key_type, text)` pairs analogous to entity multi-key indexing.
pub fn relationship_search_keys(
    source_label: &str,
    relation_type: &RelationType,
    target_label: &str,
    source_file: Option<&str>,
) -> Vec<(String, String)> {
    let base = Relationship::embed_text(source_label, relation_type, target_label);
    let mut keys = vec![
        ("relation".to_string(), base.clone()),
        (
            "interaction".to_string(),
            format!("how does {source_label} interact with {target_label}: {relation_type}"),
        ),
    ];
    if let Some(sf) = source_file.filter(|s| !s.is_empty()) {
        keys.push(("context".to_string(), format!("{base} in {sf}")));
    }
    keys
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{DomainTag, EntityType, FileType};
    use crate::model::Entity;
    use crate::relationship::Confidence;

    fn entity(name: &str, file: &str) -> Entity {
        Entity {
            id: EntityId::new(&DomainTag::Code, &EntityType::Service, name, file),
            entity_type: EntityType::Service,
            label: name.to_string(),
            source_file: Some(file.into()),
            source_location: None,
            file_type: FileType::Code,
            metadata: serde_json::json!({}),
            legacy_id: None,
            iri: None,
        }
    }

    fn rel(src: &Entity, tgt: &Entity, rt: RelationType) -> Relationship {
        Relationship {
            source: src.id.clone(),
            target: tgt.id.clone(),
            relation_type: rt,
            confidence: Confidence::Extracted,
            weight: 1.0,
            source_file: src.source_file.clone(),
            source_location: None,
            metadata: serde_json::json!({}),
            embedding: None,
        }
    }

    /// Fixture: AuthService --calls--> Database, Logger --writes--> FileStore,
    /// AuthService --depends_on--> TokenVault, Payment --calls--> Database.
    fn interaction_fixture() -> KnowledgeGraph {
        let mut kg = KnowledgeGraph::new();
        let auth = entity("AuthService", "auth.rs");
        let db = entity("Database", "db.rs");
        let logger = entity("Logger", "log.rs");
        let files = entity("FileStore", "files.rs");
        let vault = entity("TokenVault", "vault.rs");
        let pay = entity("PaymentService", "pay.rs");

        kg.add_entity(auth.clone());
        kg.add_entity(db.clone());
        kg.add_entity(logger.clone());
        kg.add_entity(files.clone());
        kg.add_entity(vault.clone());
        kg.add_entity(pay.clone());

        kg.add_relationship(rel(&auth, &db, RelationType::Calls));
        kg.add_relationship(rel(&logger, &files, RelationType::DependsOn));
        kg.add_relationship(rel(&auth, &vault, RelationType::DependsOn));
        kg.add_relationship(rel(&pay, &db, RelationType::Calls));

        kg
    }

    #[test]
    fn index_builds_one_entry_per_edge() {
        let kg = interaction_fixture();
        let idx = EdgeEmbeddingIndex::build(&kg, DEFAULT_EDGE_EMBED_DIMS);
        assert_eq!(idx.len(), 4);
        assert_eq!(idx.dims(), DEFAULT_EDGE_EMBED_DIMS);
        assert!(!idx.is_empty());
    }

    #[test]
    fn populate_writes_embedding_field() {
        let mut kg = interaction_fixture();
        for (_, _, r) in kg.edges() {
            assert!(r.embedding.is_none());
        }
        populate_edge_embeddings(&mut kg, DEFAULT_EDGE_EMBED_DIMS);
        let mut n = 0;
        for (_, _, r) in kg.edges() {
            let emb = r.embedding.as_ref().expect("embedding populated");
            assert_eq!(emb.len(), DEFAULT_EDGE_EMBED_DIMS);
            n += 1;
        }
        assert_eq!(n, 4);
    }

    #[test]
    fn interaction_query_finds_auth_database() {
        // LightRAG P5 acceptance: "how does X interact with Y?"
        let kg = interaction_fixture();
        let hits = query_relationships(&kg, "how does AuthService interact with Database", 3);
        assert!(!hits.is_empty(), "expected at least one hit");
        // Top hit should involve AuthService and Database (calls edge).
        let top = &hits[0];
        let involves_auth_db = (top.source_label == "AuthService"
            && top.target_label == "Database")
            || (top.source_label == "Database" && top.target_label == "AuthService");
        assert!(
            involves_auth_db,
            "top hit should be AuthService↔Database, got {} -[{}]-> {} (score={})",
            top.source_label, top.relation_type, top.target_label, top.score
        );
        assert_eq!(top.relation_type, "calls");
    }

    #[test]
    fn logger_file_query_prefers_logger_edge() {
        let kg = interaction_fixture();
        let hits = query_relationships(&kg, "Logger depends_on FileStore", 2);
        assert!(!hits.is_empty());
        assert_eq!(hits[0].source_label, "Logger");
        assert_eq!(hits[0].target_label, "FileStore");
    }

    #[test]
    fn hnsw_entries_use_rel_prefix() {
        let kg = interaction_fixture();
        let idx = EdgeEmbeddingIndex::build(&kg, 32);
        let entries = idx.hnsw_entries();
        assert_eq!(entries.len(), 4);
        for (id, emb, meta) in &entries {
            assert!(id.starts_with(EDGE_HNSW_PREFIX), "id={id}");
            assert_eq!(emb.len(), 32);
            assert_eq!(meta["kind"], "relationship");
            assert!(meta["relationship_id"].as_str().is_some());
        }
    }

    #[test]
    fn search_keys_include_interaction_phrasing() {
        let keys = relationship_search_keys(
            "AuthService",
            &RelationType::Calls,
            "Database",
            Some("auth.rs"),
        );
        assert!(keys.iter().any(|(k, _)| k == "relation"));
        assert!(keys.iter().any(|(k, _)| k == "interaction"));
        assert!(keys.iter().any(|(k, _)| k == "context"));
        let interaction = keys
            .iter()
            .find(|(k, _)| k == "interaction")
            .map(|(_, t)| t.as_str())
            .unwrap();
        assert!(interaction.contains("how does AuthService interact with Database"));
    }

    #[test]
    fn empty_graph_query_is_empty() {
        let kg = KnowledgeGraph::new();
        let hits = query_relationships(&kg, "anything", 5);
        assert!(hits.is_empty());
    }

    #[test]
    fn build_with_embedder_uses_custom_vectors() {
        let kg = interaction_fixture();
        let idx = EdgeEmbeddingIndex::build_with_embedder(&kg, |text| {
            // Tiny deterministic embedding: first char code + length.
            let mut v = vec![0.0f32; 8];
            if let Some(c) = text.chars().next() {
                v[0] = c as u32 as f32;
            }
            v[1] = text.len() as f32;
            v
        });
        assert_eq!(idx.len(), 4);
        assert_eq!(idx.dims(), 8);
    }

    #[test]
    fn pre_populated_embedding_is_reused() {
        let mut kg = interaction_fixture();
        // Force a known embedding on the first edge.
        let custom = vec![1.0f32; DEFAULT_EDGE_EMBED_DIMS];
        {
            let ei = kg.inner_graph().edge_indices().next().unwrap();
            if let Some(r) = kg.edge_weight_mut(ei) {
                r.embedding = Some(custom.clone());
            }
        }
        let idx = EdgeEmbeddingIndex::build(&kg, DEFAULT_EDGE_EMBED_DIMS);
        // Searching with the custom vector should rank that edge first.
        let hits = idx.search_vector(&custom, 1);
        assert_eq!(hits.len(), 1);
        assert!(hits[0].score > 0.99);
    }
}
