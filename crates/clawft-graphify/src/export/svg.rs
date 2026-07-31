//! Positioned SVG export — render a layouted knowledge graph as static SVG.
//!
//! Pipeline:
//! 1. Infer (or accept) a [`TopologySchema`]
//! 2. Run the layout engine → [`PositionedGraph`] (Barnes–Hut force / tree / …)
//! 3. Emit SVG with node shapes, edge paths, labels, and arrowheads
//!
//! This is the WEFT-359 "positioned-SVG" path: coordinates come from the Rust
//! layout engine, not from a browser physics simulation.

use std::fs;
use std::path::Path;

use crate::layout::layout_graph;
use crate::layout::positioned::{PositionedEdge, PositionedGraph, PositionedNode};
use crate::model::KnowledgeGraph;
use crate::topology::{DashStyle, NodeShape, TopologySchema};
use crate::topology_infer;
use crate::GraphifyError;

/// Default canvas size for SVG export.
pub const DEFAULT_WIDTH: f64 = 1200.0;
pub const DEFAULT_HEIGHT: f64 = 800.0;

/// Export a knowledge graph to a positioned SVG file.
///
/// Infers a topology schema from the graph and runs the full layout engine
/// (Barnes–Hut force layout when geometry is Force).
pub fn to_svg(kg: &KnowledgeGraph, output: &Path) -> Result<(), GraphifyError> {
    to_svg_with_size(kg, output, DEFAULT_WIDTH, DEFAULT_HEIGHT)
}

/// Export with an explicit canvas size.
pub fn to_svg_with_size(
    kg: &KnowledgeGraph,
    output: &Path,
    width: f64,
    height: f64,
) -> Result<(), GraphifyError> {
    let schema = topology_infer::infer_schema(kg, "svg-export");
    to_svg_with_schema(kg, &schema, output, width, height)
}

/// Export using a caller-supplied topology schema.
pub fn to_svg_with_schema(
    kg: &KnowledgeGraph,
    schema: &TopologySchema,
    output: &Path,
    width: f64,
    height: f64,
) -> Result<(), GraphifyError> {
    let svg = to_svg_string(kg, schema, width, height)?;
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| {
                GraphifyError::ExportError(format!(
                    "Failed to create dir {}: {e}",
                    parent.display()
                ))
            })?;
        }
    }
    fs::write(output, svg).map_err(|e| {
        GraphifyError::ExportError(format!("Failed to write {}: {e}", output.display()))
    })?;
    Ok(())
}

/// Layout + SVG string (no filesystem I/O).
pub fn to_svg_string(
    kg: &KnowledgeGraph,
    schema: &TopologySchema,
    width: f64,
    height: f64,
) -> Result<String, GraphifyError> {
    let positioned = layout_graph(kg, schema, width, height);
    Ok(positioned_to_svg(&positioned))
}

/// Render an already-positioned graph to SVG.
pub fn positioned_to_svg(pg: &PositionedGraph) -> String {
    let vp = &pg.viewport;
    let mut out = String::with_capacity(4096 + pg.nodes.len() * 160 + pg.edges.len() * 120);

    out.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    out.push('\n');
    out.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="{vx:.2} {vy:.2} {vw:.2} {vh:.2}" width="{ww:.0}" height="{hh:.0}" data-schema="{schema}" data-schema-version="{ver}">"#,
        vx = vp.x,
        vy = vp.y,
        vw = vp.width.max(1.0),
        vh = vp.height.max(1.0),
        ww = vp.width.max(1.0),
        hh = vp.height.max(1.0),
        schema = xml_escape(&pg.schema_name),
        ver = xml_escape(&pg.schema_version),
    ));
    out.push('\n');

    // Styles + arrow marker.
    out.push_str(
        r##"  <defs>
    <marker id="arrowhead" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto" markerUnits="strokeWidth">
      <polygon points="0 0, 10 3.5, 0 7" fill="#666666"/>
    </marker>
    <style type="text/css"><![CDATA[
      .edge { fill: none; }
      .node-label { font-family: system-ui, -apple-system, sans-serif; font-size: 11px; fill: #1a1a1a; text-anchor: middle; dominant-baseline: central; pointer-events: none; }
      .edge-label { font-family: system-ui, -apple-system, sans-serif; font-size: 9px; fill: #555555; text-anchor: middle; }
      .bg { fill: #fafafa; }
    ]]></style>
  </defs>
"##,
    );

    // Background.
    out.push_str(&format!(
        r#"  <rect class="bg" x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}"/>"#,
        vp.x,
        vp.y,
        vp.width.max(1.0),
        vp.height.max(1.0)
    ));
    out.push('\n');

    // Edges under nodes.
    out.push_str(r#"  <g class="edges">"#);
    out.push('\n');
    for edge in &pg.edges {
        render_edge(&mut out, edge);
    }
    out.push_str("  </g>\n");

    // Nodes.
    out.push_str(r#"  <g class="nodes">"#);
    out.push('\n');
    for node in &pg.nodes {
        render_node(&mut out, node);
    }
    out.push_str("  </g>\n");

    out.push_str("</svg>\n");
    out
}

fn render_edge(out: &mut String, edge: &PositionedEdge) {
    if edge.path.len() < 2 {
        return;
    }
    let start = edge.path[0];
    let end = edge.path[edge.path.len() - 1];

    // Shorten the line slightly so arrowheads sit on the node boundary.
    let (x1, y1, x2, y2) = trim_segment(start[0], start[1], end[0], end[1], 14.0);

    let dash = match edge.dash {
        DashStyle::Solid => "",
        DashStyle::Dashed => r#" stroke-dasharray="6 4""#,
        DashStyle::Dotted => r#" stroke-dasharray="2 3""#,
    };
    let marker = if edge.arrow {
        r#" marker-end="url(#arrowhead)""#
    } else {
        ""
    };
    let animated = if edge.animated {
        r#" data-animated="true""#
    } else {
        ""
    };

    if edge.path.len() == 2 {
        out.push_str(&format!(
            r#"    <line class="edge" data-source="{src}" data-target="{tgt}" data-type="{ty}" x1="{x1:.2}" y1="{y1:.2}" x2="{x2:.2}" y2="{y2:.2}" stroke="{stroke}" stroke-width="{w:.2}"{dash}{marker}{animated}/>"#,
            src = xml_escape(&edge.source_id),
            tgt = xml_escape(&edge.target_id),
            ty = xml_escape(&edge.edge_type),
            stroke = xml_escape(&edge.stroke),
            w = edge.width.max(0.5),
        ));
    } else {
        // Polyline path for multi-point edges.
        let mut d = format!("M {:.2} {:.2}", edge.path[0][0], edge.path[0][1]);
        for p in &edge.path[1..] {
            d.push_str(&format!(" L {:.2} {:.2}", p[0], p[1]));
        }
        out.push_str(&format!(
            r#"    <path class="edge" data-source="{src}" data-target="{tgt}" data-type="{ty}" d="{d}" stroke="{stroke}" stroke-width="{w:.2}"{dash}{marker}{animated}/>"#,
            src = xml_escape(&edge.source_id),
            tgt = xml_escape(&edge.target_id),
            ty = xml_escape(&edge.edge_type),
            stroke = xml_escape(&edge.stroke),
            w = edge.width.max(0.5),
        ));
    }
    out.push('\n');

    if let Some(label) = &edge.label {
        if !label.is_empty() {
            let mx = (x1 + x2) * 0.5;
            let my = (y1 + y2) * 0.5 - 4.0;
            out.push_str(&format!(
                r#"    <text class="edge-label" x="{mx:.2}" y="{my:.2}">{lbl}</text>"#,
                lbl = xml_escape(label),
            ));
            out.push('\n');
        }
    }
}

fn render_node(out: &mut String, node: &PositionedNode) {
    let half_w = node.width.max(8.0) * 0.5;
    let half_h = node.height.max(8.0) * 0.5;
    let color = xml_escape(&node.color);
    let id = xml_escape(&node.id);
    let ntype = xml_escape(&node.node_type);
    let label = xml_escape(&node.label);

    out.push_str(&format!(
        r#"    <g class="node" data-id="{id}" data-type="{ntype}">"#
    ));
    out.push('\n');

    match node.shape {
        NodeShape::Circle => {
            let r = half_w.min(half_h);
            out.push_str(&format!(
                r##"      <circle cx="{cx:.2}" cy="{cy:.2}" r="{r:.2}" fill="{color}" stroke="#333333" stroke-width="1.2"/>"##,
                cx = node.x,
                cy = node.y,
            ));
        }
        NodeShape::Rect => {
            out.push_str(&format!(
                r##"      <rect x="{x:.2}" y="{y:.2}" width="{w:.2}" height="{h:.2}" rx="4" ry="4" fill="{color}" stroke="#333333" stroke-width="1.2"/>"##,
                x = node.x - half_w,
                y = node.y - half_h,
                w = half_w * 2.0,
                h = half_h * 2.0,
            ));
        }
        NodeShape::Diamond => {
            let points = format!(
                "{cx:.2},{top:.2} {right:.2},{cy:.2} {cx:.2},{bottom:.2} {left:.2},{cy:.2}",
                cx = node.x,
                cy = node.y,
                top = node.y - half_h,
                bottom = node.y + half_h,
                left = node.x - half_w,
                right = node.x + half_w,
            );
            out.push_str(&format!(
                r##"      <polygon points="{points}" fill="{color}" stroke="#333333" stroke-width="1.2"/>"##
            ));
        }
        NodeShape::Hexagon => {
            let r = half_w.min(half_h);
            let pts: Vec<String> = (0..6)
                .map(|i| {
                    let a = std::f64::consts::PI / 6.0 + i as f64 * std::f64::consts::PI / 3.0;
                    format!(
                        "{:.2},{:.2}",
                        node.x + r * a.cos(),
                        node.y + r * a.sin()
                    )
                })
                .collect();
            out.push_str(&format!(
                r##"      <polygon points="{pts}" fill="{color}" stroke="#333333" stroke-width="1.2"/>"##,
                pts = pts.join(" "),
            ));
        }
        NodeShape::Pill => {
            let w = half_w * 2.0;
            let h = half_h * 2.0;
            let rx = h * 0.5;
            out.push_str(&format!(
                r##"      <rect x="{x:.2}" y="{y:.2}" width="{w:.2}" height="{h:.2}" rx="{rx:.2}" ry="{rx:.2}" fill="{color}" stroke="#333333" stroke-width="1.2"/>"##,
                x = node.x - half_w,
                y = node.y - half_h,
            ));
        }
    }
    out.push('\n');

    // Subgraph indicator (expandable).
    if node.has_subgraph {
        out.push_str(&format!(
            r##"      <circle cx="{cx:.2}" cy="{cy:.2}" r="3.5" fill="#ffffff" stroke="#333333" stroke-width="1"/>"##,
            cx = node.x + half_w * 0.55,
            cy = node.y - half_h * 0.55,
        ));
        out.push('\n');
    }

    // Label below the node.
    let label_y = node.y + half_h + 12.0;
    out.push_str(&format!(
        r#"      <text class="node-label" x="{cx:.2}" y="{ly:.2}">{label}</text>"#,
        cx = node.x,
        ly = label_y,
    ));
    out.push('\n');

    if let Some(iri) = &node.iri {
        out.push_str(&format!(
            r#"      <title>{iri}</title>"#,
            iri = xml_escape(iri)
        ));
        out.push('\n');
    }

    out.push_str("    </g>\n");
}

/// Pull endpoints inward along the segment so lines meet node borders.
fn trim_segment(x1: f64, y1: f64, x2: f64, y2: f64, pad: f64) -> (f64, f64, f64, f64) {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len = (dx * dx + dy * dy).sqrt();
    if len < pad * 2.0 + 1.0 {
        return (x1, y1, x2, y2);
    }
    let ux = dx / len;
    let uy = dy / len;
    (
        x1 + ux * pad,
        y1 + uy * pad,
        x2 - ux * pad,
        y2 - uy * pad,
    )
}

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c if c.is_control() && c != '\t' && c != '\n' && c != '\r' => {}
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{DomainTag, EntityId, EntityType, FileType};
    use crate::model::Entity;
    use crate::relationship::{Confidence, RelationType, Relationship};
    fn entity(name: &str, etype: EntityType) -> Entity {
        Entity {
            id: EntityId::new(&DomainTag::Code, &etype, name, "test.rs"),
            entity_type: etype,
            label: name.to_string(),
            iri: None,
            source_file: Some("test.rs".into()),
            source_location: None,
            file_type: FileType::Code,
            metadata: serde_json::json!({}),
            legacy_id: None,
        }
    }

    fn force_schema() -> TopologySchema {
        TopologySchema::from_yaml(
            r##"
name: svg-test
label: "SVG Test"
version: "1.0.0"
nodes:
  function:
    geometry: force
    style:
      shape: circle
      color: "#22c55e"
  module:
    geometry: force
    style:
      shape: rect
      color: "#8b5cf6"
  "*":
    geometry: force
edges:
  - type: calls
    from: function
    to: function
  - type: contains
    from: module
    to: function
modes:
  structure:
    root_geometry: force
"##,
        )
        .unwrap()
    }

    #[test]
    fn svg_contains_root_and_positions() {
        let mut kg = KnowledgeGraph::new();
        let a = entity("foo", EntityType::Function);
        let b = entity("bar", EntityType::Function);
        kg.add_entity(a.clone());
        kg.add_entity(b.clone());
        kg.add_relationship(Relationship {
            source: a.id.clone(),
            target: b.id.clone(),
            relation_type: RelationType::Calls,
            confidence: Confidence::Extracted,
            weight: 1.0,
            source_file: None,
            source_location: None,
            metadata: serde_json::json!({}),
        });

        let schema = force_schema();
        let svg = to_svg_string(&kg, &schema, 800.0, 600.0).unwrap();

        assert!(svg.contains("<svg"), "missing svg root");
        assert!(svg.contains("xmlns=\"http://www.w3.org/2000/svg\""));
        assert!(svg.contains("class=\"node\""), "missing nodes");
        assert!(svg.contains("class=\"edge\""), "missing edges");
        assert!(svg.contains("foo") && svg.contains("bar"), "labels missing");
        assert!(svg.contains("viewBox="), "missing viewBox from positioned viewport");
        // Circle nodes for functions.
        assert!(svg.contains("<circle"), "expected circle shapes");
        // Marker for arrows.
        assert!(svg.contains("arrowhead"));
    }

    #[test]
    fn svg_export_writes_file() {
        let mut kg = KnowledgeGraph::new();
        kg.add_entity(entity("solo", EntityType::Function));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out").join("graph.svg");
        to_svg_with_schema(&kg, &force_schema(), &path, 400.0, 300.0).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.starts_with("<?xml"));
        assert!(content.contains("solo"));
    }

    #[test]
    fn svg_empty_graph() {
        let kg = KnowledgeGraph::new();
        let svg = to_svg_string(&kg, &force_schema(), 400.0, 300.0).unwrap();
        assert!(svg.contains("<svg"));
        assert!(!svg.contains("class=\"node\""));
    }

    #[test]
    fn svg_escapes_labels() {
        let mut kg = KnowledgeGraph::new();
        let mut e = entity("x", EntityType::Function);
        e.label = r#"a<b>&"c"#.into();
        kg.add_entity(e);
        let svg = to_svg_string(&kg, &force_schema(), 400.0, 300.0).unwrap();
        assert!(svg.contains("&lt;") || svg.contains("&amp;"));
        assert!(!svg.contains("a<b>"));
    }

    #[test]
    fn positioned_to_svg_uses_node_coordinates() {
        use crate::layout::positioned::{PositionedGraph, PositionedNode, Rect};

        let pg = PositionedGraph {
            nodes: vec![PositionedNode {
                id: "n1".into(),
                x: 100.0,
                y: 200.0,
                width: 40.0,
                height: 40.0,
                label: "N1".into(),
                node_type: "function".into(),
                iri: None,
                shape: NodeShape::Circle,
                color: "#ff0000".into(),
                icon: None,
                has_subgraph: false,
                disposition: None,
                metrics: Default::default(),
            }],
            edges: vec![],
            viewport: Rect {
                x: 0.0,
                y: 0.0,
                width: 400.0,
                height: 300.0,
            },
            schema_name: "t".into(),
            schema_version: "1".into(),
        };
        let svg = positioned_to_svg(&pg);
        assert!(svg.contains(r#"cx="100.00""#) || svg.contains(r#"cx="100""#));
        assert!(svg.contains(r#"cy="200.00""#) || svg.contains(r#"cy="200""#));
        assert!(svg.contains("#ff0000"));
    }

    #[test]
    fn to_svg_default_infers_schema() {
        let mut kg = KnowledgeGraph::new();
        let a = entity("a", EntityType::Function);
        let b = entity("b", EntityType::Function);
        kg.add_entity(a.clone());
        kg.add_entity(b.clone());
        kg.add_relationship(Relationship {
            source: a.id,
            target: b.id,
            relation_type: RelationType::Calls,
            confidence: Confidence::Extracted,
            weight: 1.0,
            source_file: None,
            source_location: None,
            metadata: serde_json::json!({}),
        });
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("g.svg");
        to_svg(&kg, &path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("class=\"edge\""));
        assert!(content.contains("data-schema="));
    }
}
