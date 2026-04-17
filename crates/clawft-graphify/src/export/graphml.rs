//! GraphML XML export.
//!
//! Generates standard GraphML that can be imported by Gephi and yEd.
//! Adds `community` as a node attribute.

use crate::analyze::node_community_map;
use crate::export::sanitize_label;
use crate::model::{EntityId, KnowledgeGraph};
use crate::GraphifyError;
use std::collections::HashMap;
use std::path::Path;

/// Escape a string for safe XML embedding.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Export the knowledge graph to GraphML format.
pub fn to_graphml(
    kg: &KnowledgeGraph,
    communities: &HashMap<usize, Vec<EntityId>>,
    output: &Path,
) -> Result<(), GraphifyError> {
    let node_community = node_community_map(communities);

    let mut lines: Vec<String> = Vec::new();
    lines.push(r#"<?xml version="1.0" encoding="UTF-8"?>"#.to_owned());
    lines.push(
        r#"<graphml xmlns="http://graphml.graphstruct.org/graphml" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://graphml.graphstruct.org/graphml http://graphml.graphstruct.org/graphml/1.0/graphml.xsd">"#
            .to_owned(),
    );

    // Attribute declarations
    lines.push(r#"  <key id="label" for="node" attr.name="label" attr.type="string"/>"#.to_owned());
    lines.push(
        r#"  <key id="file_type" for="node" attr.name="file_type" attr.type="string"/>"#.to_owned(),
    );
    lines.push(
        r#"  <key id="source_file" for="node" attr.name="source_file" attr.type="string"/>"#
            .to_owned(),
    );
    lines.push(
        r#"  <key id="community" for="node" attr.name="community" attr.type="int"/>"#.to_owned(),
    );
    lines.push(
        r#"  <key id="relation" for="edge" attr.name="relation" attr.type="string"/>"#.to_owned(),
    );
    lines.push(
        r#"  <key id="confidence" for="edge" attr.name="confidence" attr.type="string"/>"#
            .to_owned(),
    );

    lines.push(r#"  <graph id="G" edgemode="undirected">"#.to_owned());

    // Nodes
    for entity in kg.entities.values() {
        let id = xml_escape(&entity.id.0);
        let label = xml_escape(&sanitize_label(&entity.label));
        let file_type = xml_escape(&entity.file_type);
        let source_file = xml_escape(&entity.source_file);
        let community = node_community.get(&entity.id).copied().unwrap_or(0);

        lines.push(format!(r#"    <node id="{id}">"#));
        lines.push(format!(r#"      <data key="label">{label}</data>"#));
        lines.push(format!(r#"      <data key="file_type">{file_type}</data>"#));
        lines.push(format!(r#"      <data key="source_file">{source_file}</data>"#));
        lines.push(format!(r#"      <data key="community">{community}</data>"#));
        lines.push(r#"    </node>"#.to_owned());
    }

    // Edges
    for (i, rel) in kg.relationships.iter().enumerate() {
        let source = xml_escape(&rel.source.0);
        let target = xml_escape(&rel.target.0);
        let relation = xml_escape(&rel.relation);
        let confidence = rel.confidence.as_str();

        lines.push(format!(r#"    <edge id="e{i}" source="{source}" target="{target}">"#));
        lines.push(format!(r#"      <data key="relation">{relation}</data>"#));
        lines.push(format!(r#"      <data key="confidence">{confidence}</data>"#));
        lines.push(r#"    </edge>"#.to_owned());
    }

    lines.push(r#"  </graph>"#.to_owned());
    lines.push(r#"</graphml>"#.to_owned());

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output, lines.join("\n"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Confidence, Entity, Relationship};

    fn entity(id: &str) -> Entity {
        Entity {
            id: EntityId::new(id),
            label: id.to_owned(),
            file_type: "code".to_owned(),
            source_file: format!("{id}.py"),
            source_location: String::new(),
            community: None,
            metadata: Default::default(),
            iri: None,
        }
    }

    fn rel(src: &str, tgt: &str) -> Relationship {
        Relationship {
            source: EntityId::new(src),
            target: EntityId::new(tgt),
            relation: "calls".to_owned(),
            confidence: Confidence::Extracted,
            confidence_score: 1.0,
            weight: 1.0,
            source_file: String::new(),
            metadata: Default::default(),
        }
    }

    #[test]
    fn to_graphml_creates_valid_xml() {
        let kg = KnowledgeGraph::new(
            vec![entity("a"), entity("b")],
            vec![rel("a", "b")],
            vec![],
        );
        let communities =
            HashMap::from([(0, vec![EntityId::new("a"), EntityId::new("b")])]);

        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("graph.graphml");
        to_graphml(&kg, &communities, &output).unwrap();
        assert!(output.exists());

        let content = std::fs::read_to_string(&output).unwrap();
        assert!(content.contains("<?xml"));
        assert!(content.contains("<graphml"));
        assert!(content.contains("community"));
        assert!(content.contains(r#"<node id="a">"#) || content.contains(r#"<node id="b">"#));
    }

    #[test]
    fn to_graphml_escapes_special_chars() {
        let mut e = entity("x&y");
        e.label = "foo<bar>".to_owned();
        let kg = KnowledgeGraph::new(vec![e], vec![], vec![]);

        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("graph.graphml");
        to_graphml(&kg, &HashMap::new(), &output).unwrap();

        let content = std::fs::read_to_string(&output).unwrap();
        // Node ID is xml_escape'd directly
        assert!(content.contains("x&amp;y"));
        // Label goes through sanitize_label (HTML-escaping) then xml_escape,
        // so `<` -> `&lt;` -> `&amp;lt;`
        assert!(content.contains("&amp;lt;") || content.contains("&lt;"));
    }
}
