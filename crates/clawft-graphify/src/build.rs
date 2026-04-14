//! Graph assembly: merge multiple `ExtractionResult`s into a single
//! `KnowledgeGraph`.
//!
//! Implements the same deduplication and edge-cleaning semantics as
//! Python's `build.py`.

use crate::model::{Entity, ExtractionResult, KnowledgeGraph};
use crate::relationship::Relationship;
use crate::validation::validate_extraction;
use crate::GraphifyError;

/// Chain event kind for graph build completion.
pub const EVENT_KIND_GRAPHIFY_BUILD: &str = "graphify.build";

/// Merge multiple extraction results into a single `KnowledgeGraph`.
///
/// Entities with the same ID are deduplicated (last-write-wins). Relationships
/// whose source or target is not in the graph are silently dropped (expected
/// for external/stdlib imports).
pub fn build(extractions: &[ExtractionResult]) -> KnowledgeGraph {
    let mut kg = KnowledgeGraph::new();

    // Accumulate stats.
    for ext in extractions {
        kg.stats.files_processed += 1;
        kg.stats.input_tokens += ext.input_tokens;
        kg.stats.output_tokens += ext.output_tokens;

        // Add entities (last-write-wins for duplicates).
        for entity in &ext.entities {
            kg.add_entity(entity.clone());
        }

        // Add relationships (skips if source/target missing).
        for rel in &ext.relationships {
            kg.add_relationship(rel.clone());
        }

        // Collect hyperedges.
        kg.hyperedges.extend(ext.hyperedges.iter().cloned());
    }

    kg.stats.entities_extracted = kg.entity_count();
    kg.stats.relationships_extracted = kg.relationship_count();

    // Chain event marker -- daemon subscriber forwards to ExoChain.
    tracing::info!(
        target: "chain_event",
        source = "graphify",
        kind = EVENT_KIND_GRAPHIFY_BUILD,
        entity_count = kg.entity_count(),
        relationship_count = kg.relationship_count(),
        files_processed = kg.stats.files_processed,
        "chain"
    );

    kg
}

/// Build a `KnowledgeGraph` from a JSON extraction value (Python-compatible).
///
/// Expects the same schema as Python's extraction output:
/// `{ "nodes": [...], "edges": [...], "hyperedges": [...] }`.
pub fn build_from_json(data: &serde_json::Value) -> Result<KnowledgeGraph, GraphifyError> {
    use crate::entity::{EntityId, EntityType, FileType};
    use crate::relationship::{Confidence, RelationType};

    let errors = validate_extraction(data);
    // Filter out dangling-edge warnings -- those are expected.
    let real_errors: Vec<_> = errors
        .iter()
        .filter(|e| !e.contains("does not match any node id"))
        .collect();
    if !real_errors.is_empty() {
        return Err(GraphifyError::ValidationError(format!(
            "{} schema error(s): {}",
            real_errors.len(),
            real_errors[0]
        )));
    }

    let mut kg = KnowledgeGraph::new();

    // Parse nodes.
    if let Some(nodes) = data.get("nodes").and_then(|v| v.as_array()) {
        for node in nodes {
            let legacy_id = node
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let label = node
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let source_file = node.get("source_file").and_then(|v| v.as_str()).map(String::from);
            let source_location = node.get("source_location").and_then(|v| v.as_str()).map(String::from);
            let file_type_str = node
                .get("file_type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let file_type = FileType::from_str_loose(file_type_str).unwrap_or(FileType::Unknown);

            let id = EntityId::from_legacy_string(&legacy_id);
            let entity = Entity {
                id,
                entity_type: EntityType::Custom(
                    node.get("entity_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("concept")
                        .to_string(),
                ),
                label,
                source_file,
                source_location,
                file_type,
                metadata: node.get("metadata").cloned().unwrap_or(serde_json::json!({})),
                legacy_id: Some(legacy_id),
            };
            kg.add_entity(entity);
        }
    }

    // Parse edges.
    if let Some(edges) = data.get("edges").and_then(|v| v.as_array()) {
        for edge in edges {
            let src_str = edge.get("source").and_then(|v| v.as_str()).unwrap_or("");
            let tgt_str = edge.get("target").and_then(|v| v.as_str()).unwrap_or("");
            let relation_str = edge
                .get("relation")
                .and_then(|v| v.as_str())
                .unwrap_or("related_to");
            let confidence_str = edge
                .get("confidence")
                .and_then(|v| v.as_str())
                .unwrap_or("EXTRACTED");
            let weight = edge
                .get("weight")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0) as f32;

            let confidence = Confidence::from_str_loose(confidence_str).unwrap_or(Confidence::Extracted);
            let relation_type = match relation_str {
                "calls" => RelationType::Calls,
                "imports" => RelationType::Imports,
                "imports_from" => RelationType::ImportsFrom,
                "depends_on" => RelationType::DependsOn,
                "contains" => RelationType::Contains,
                "implements" => RelationType::Implements,
                "configures" => RelationType::Configures,
                "extends" => RelationType::Extends,
                "method_of" => RelationType::MethodOf,
                "instantiates" => RelationType::Instantiates,
                "related_to" => RelationType::RelatedTo,
                "case_of" => RelationType::CaseOf,
                other => RelationType::Custom(other.to_string()),
            };

            let rel = Relationship {
                source: EntityId::from_legacy_string(src_str),
                target: EntityId::from_legacy_string(tgt_str),
                relation_type,
                confidence,
                weight,
                source_file: edge.get("source_file").and_then(|v| v.as_str()).map(String::from),
                source_location: edge.get("source_location").and_then(|v| v.as_str()).map(String::from),
                metadata: serde_json::json!({
                    "_src": src_str,
                    "_tgt": tgt_str,
                }),
            };
            kg.add_relationship(rel);
        }
    }

    // Parse hyperedges.
    if let Some(hes) = data.get("hyperedges").and_then(|v| v.as_array()) {
        for he in hes {
            let label = he
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let entity_ids = he
                .get("nodes")
                .or_else(|| he.get("entity_ids"))
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(EntityId::from_legacy_string)
                        .collect()
                })
                .unwrap_or_default();
            kg.hyperedges.push(crate::model::Hyperedge {
                label,
                entity_ids,
                metadata: he.get("metadata").cloned().unwrap_or(serde_json::json!({})),
            });
        }
    }

    Ok(kg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{DomainTag, EntityId, EntityType, FileType};
    use crate::model::Entity;
    use crate::relationship::{Confidence, RelationType};

    fn sample_extraction(name: &str) -> ExtractionResult {
        let e1 = Entity {
            id: EntityId::new(&DomainTag::Code, &EntityType::Module, name, &format!("{name}.py")),
            entity_type: EntityType::Module,
            label: name.into(),
            source_file: Some(format!("{name}.py")),
            source_location: Some("L1".into()),
            file_type: FileType::Code,
            metadata: serde_json::json!({}),
            legacy_id: Some(name.into()),
        };
        ExtractionResult {
            source_file: format!("{name}.py"),
            entities: vec![e1],
            relationships: vec![],
            hyperedges: vec![],
            input_tokens: 100,
            output_tokens: 50,
            errors: vec![],
        }
    }

    #[test]
    fn merge_two_extractions() {
        let ext1 = sample_extraction("auth");
        let ext2 = sample_extraction("db");
        let kg = build(&[ext1, ext2]);
        assert_eq!(kg.entity_count(), 2);
        assert_eq!(kg.stats.files_processed, 2);
        assert_eq!(kg.stats.input_tokens, 200);
    }

    #[test]
    fn dedup_entities_last_wins() {
        let mut ext1 = sample_extraction("auth");
        let mut ext2 = sample_extraction("auth");
        ext1.entities[0].label = "first".into();
        ext2.entities[0].label = "second".into();
        let kg = build(&[ext1, ext2]);
        assert_eq!(kg.entity_count(), 1);
        let entity = kg.entities().next().unwrap();
        assert_eq!(entity.label, "second");
    }

    #[test]
    fn external_edges_silently_dropped() {
        let mut ext = sample_extraction("app");
        let external_id = EntityId::new(&DomainTag::Code, &EntityType::Import, "os", "stdlib");
        ext.relationships.push(Relationship {
            source: ext.entities[0].id.clone(),
            target: external_id,
            relation_type: RelationType::Imports,
            confidence: Confidence::Extracted,
            weight: 1.0,
            source_file: Some("app.py".into()),
            source_location: Some("L5".into()),
            metadata: serde_json::json!({}),
        });
        let kg = build(&[ext]);
        assert_eq!(kg.entity_count(), 1);
        assert_eq!(kg.relationship_count(), 0);
    }

    #[test]
    fn build_from_json_basic() {
        let data = serde_json::json!({
            "nodes": [
                {"id": "auth", "label": "auth", "file_type": "code", "source_file": "auth.py"},
                {"id": "db", "label": "db", "file_type": "code", "source_file": "db.py"},
            ],
            "edges": [
                {"source": "auth", "target": "db", "relation": "imports", "confidence": "EXTRACTED", "source_file": "auth.py", "weight": 1.0},
            ],
            "hyperedges": [],
        });
        let kg = build_from_json(&data).unwrap();
        assert_eq!(kg.entity_count(), 2);
        assert_eq!(kg.relationship_count(), 1);
    }
}
