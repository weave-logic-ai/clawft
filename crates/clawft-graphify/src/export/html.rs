//! Interactive HTML/vis.js graph visualization export.
//!
//! Generates a self-contained HTML file with:
//! - vis.js forceAtlas2Based physics layout
//! - Node sizing by degree, community coloring
//! - Click-to-inspect sidebar, search with autocomplete
//! - Community legend with toggle, neighbor navigation
//! - Hyperedge rendering as convex hull polygons
//! - XSS prevention via label sanitization

use crate::analyze::node_community_map;
use crate::eml_models::LayoutModel;
use crate::export::{sanitize_label, COMMUNITY_COLORS};
use crate::model::{EntityId, KnowledgeGraph};
use crate::GraphifyError;
use std::collections::HashMap;
use std::path::Path;

/// Maximum nodes for HTML visualization.
pub const MAX_NODES_FOR_VIZ: usize = 5_000;

/// Generate an interactive vis.js HTML visualization.
pub fn to_html(
    kg: &KnowledgeGraph,
    communities: &HashMap<usize, Vec<EntityId>>,
    community_labels: &HashMap<usize, String>,
    output: &Path,
) -> Result<(), GraphifyError> {
    to_html_eml(kg, communities, community_labels, output, None)
}

/// Generate an interactive vis.js HTML visualization with optional EML layout model.
///
/// When `eml_model` is `Some` and trained, uses learned ForceAtlas2 physics
/// parameters. Pass `None` to use the original hardcoded values.
pub fn to_html_eml(
    kg: &KnowledgeGraph,
    communities: &HashMap<usize, Vec<EntityId>>,
    community_labels: &HashMap<usize, String>,
    output: &Path,
    eml_model: Option<&LayoutModel>,
) -> Result<(), GraphifyError> {
    if kg.node_count() > MAX_NODES_FOR_VIZ {
        return Err(GraphifyError::TooLarge(format!(
            "Graph has {} nodes - too large for HTML viz. Use --no-viz or reduce input size.",
            kg.node_count()
        )));
    }

    let node_community = node_community_map(communities);
    let max_deg = kg
        .entity_ids()
        .map(|id| kg.degree(id))
        .max()
        .unwrap_or(1)
        .max(1);

    // Build vis.js node data
    let vis_nodes: Vec<serde_json::Value> = kg
        .entities
        .values()
        .map(|e| {
            let cid = node_community.get(&e.id).copied().unwrap_or(0);
            let color = COMMUNITY_COLORS[cid % COMMUNITY_COLORS.len()];
            let label = sanitize_label(&e.label);
            let deg = kg.degree(&e.id);
            let size = 10.0 + 30.0 * (deg as f64 / max_deg as f64);
            let font_size = if deg as f64 >= max_deg as f64 * 0.15 {
                12
            } else {
                0
            };

            let comm_name = sanitize_label(
                community_labels
                    .get(&cid)
                    .map(|s| s.as_str())
                    .unwrap_or(&format!("Community {cid}")),
            );

            serde_json::json!({
                "id": e.id.0,
                "label": label,
                "color": { "background": color, "border": color, "highlight": { "background": "#ffffff", "border": color } },
                "size": (size * 10.0).round() / 10.0,
                "font": { "size": font_size, "color": "#ffffff" },
                "title": label,
                "community": cid,
                "community_name": comm_name,
                "source_file": sanitize_label(&e.source_file),
                "file_type": &e.file_type,
                "degree": deg,
            })
        })
        .collect();

    // Build vis.js edge data
    let vis_edges: Vec<serde_json::Value> = kg
        .relationships
        .iter()
        .map(|r| {
            let is_extracted = r.confidence == crate::model::Confidence::Extracted;
            serde_json::json!({
                "from": r.source.0,
                "to": r.target.0,
                "label": &r.relation,
                "title": format!("{} [{}]", r.relation, r.confidence.as_str()),
                "dashes": !is_extracted,
                "width": if is_extracted { 2 } else { 1 },
                "color": { "opacity": if is_extracted { 0.7 } else { 0.35 } },
                "confidence": r.confidence.as_str(),
            })
        })
        .collect();

    // Build legend data
    let mut legend_data: Vec<serde_json::Value> = Vec::new();
    let mut cids: Vec<usize> = community_labels.keys().copied().collect();
    cids.sort();
    for cid in cids {
        let color = COMMUNITY_COLORS[cid % COMMUNITY_COLORS.len()];
        let lbl = community_labels
            .get(&cid)
            .cloned()
            .unwrap_or_else(|| format!("Community {cid}"));
        let n = communities.get(&cid).map(|v| v.len()).unwrap_or(0);
        legend_data.push(serde_json::json!({
            "cid": cid, "color": color, "label": lbl, "count": n
        }));
    }

    let nodes_json = serde_json::to_string(&vis_nodes).unwrap_or_default();
    let edges_json = serde_json::to_string(&vis_edges).unwrap_or_default();
    let legend_json = serde_json::to_string(&legend_data).unwrap_or_default();

    let hyperedges: Vec<serde_json::Value> = kg
        .hyperedges
        .iter()
        .map(|h| {
            serde_json::json!({
                "label": sanitize_label(&h.label),
                "nodes": h.nodes.iter().map(|n| &n.0).collect::<Vec<_>>(),
            })
        })
        .collect();
    let hyperedges_json = serde_json::to_string(&hyperedges).unwrap_or_default();

    let title = sanitize_label(&output.display().to_string());
    let stats = format!(
        "{} nodes &middot; {} edges &middot; {} communities",
        kg.node_count(),
        kg.edge_count(),
        communities.len()
    );

    let html = format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>graphify - {title}</title>
<script src="https://unpkg.com/vis-network/standalone/umd/vis-network.min.js"></script>
{styles}
</head>
<body>
<div id="graph"></div>
<div id="sidebar">
  <div id="search-wrap">
    <input id="search" type="text" placeholder="Search nodes..." autocomplete="off">
    <div id="search-results"></div>
  </div>
  <div id="info-panel">
    <h3>Node Info</h3>
    <div id="info-content"><span class="empty">Click a node to inspect it</span></div>
  </div>
  <div id="legend-wrap">
    <h3>Communities</h3>
    <div id="legend"></div>
  </div>
  <div id="stats">{stats}</div>
</div>
{main_script}
{hyperedge_script}
</body>
</html>"##,
        title = title,
        styles = HTML_STYLES,
        stats = stats,
        main_script = build_main_script(&nodes_json, &edges_json, &legend_json, eml_model, kg),
        hyperedge_script = build_hyperedge_script(&hyperedges_json),
    );

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output, html)?;
    Ok(())
}

const HTML_STYLES: &str = r#"<style>
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body { background: #0f0f1a; color: #e0e0e0; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; display: flex; height: 100vh; overflow: hidden; }
  #graph { flex: 1; }
  #sidebar { width: 280px; background: #1a1a2e; border-left: 1px solid #2a2a4e; display: flex; flex-direction: column; overflow: hidden; }
  #search-wrap { padding: 12px; border-bottom: 1px solid #2a2a4e; }
  #search { width: 100%; background: #0f0f1a; border: 1px solid #3a3a5e; color: #e0e0e0; padding: 7px 10px; border-radius: 6px; font-size: 13px; outline: none; }
  #search:focus { border-color: #4E79A7; }
  #search-results { max-height: 140px; overflow-y: auto; padding: 4px 12px; border-bottom: 1px solid #2a2a4e; display: none; }
  .search-item { padding: 4px 6px; cursor: pointer; border-radius: 4px; font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .search-item:hover { background: #2a2a4e; }
  #info-panel { padding: 14px; border-bottom: 1px solid #2a2a4e; min-height: 140px; }
  #info-panel h3 { font-size: 13px; color: #aaa; margin-bottom: 8px; text-transform: uppercase; letter-spacing: 0.05em; }
  #info-content { font-size: 13px; color: #ccc; line-height: 1.6; }
  #info-content .field { margin-bottom: 5px; }
  #info-content .field b { color: #e0e0e0; }
  #info-content .empty { color: #555; font-style: italic; }
  .neighbor-link { display: block; padding: 2px 6px; margin: 2px 0; border-radius: 3px; cursor: pointer; font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; border-left: 3px solid #333; }
  .neighbor-link:hover { background: #2a2a4e; }
  #neighbors-list { max-height: 160px; overflow-y: auto; margin-top: 4px; }
  #legend-wrap { flex: 1; overflow-y: auto; padding: 12px; }
  #legend-wrap h3 { font-size: 13px; color: #aaa; margin-bottom: 10px; text-transform: uppercase; letter-spacing: 0.05em; }
  .legend-item { display: flex; align-items: center; gap: 8px; padding: 4px 0; cursor: pointer; border-radius: 4px; font-size: 12px; }
  .legend-item:hover { background: #2a2a4e; padding-left: 4px; }
  .legend-item.dimmed { opacity: 0.35; }
  .legend-dot { width: 12px; height: 12px; border-radius: 50%; flex-shrink: 0; }
  .legend-label { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .legend-count { color: #666; font-size: 11px; }
  #stats { padding: 10px 14px; border-top: 1px solid #2a2a4e; font-size: 11px; color: #555; }
</style>"#;

fn build_main_script(
    nodes_json: &str,
    edges_json: &str,
    legend_json: &str,
    eml_model: Option<&LayoutModel>,
    kg: &KnowledgeGraph,
) -> String {
    // Resolve physics parameters from EML model or hardcoded defaults.
    use crate::eml_models::PhysicsParams;
    let params = match eml_model {
        Some(model) if model.is_trained() => {
            let n = kg.node_count() as f64;
            let e = kg.edge_count() as f64;
            let density = if n > 1.0 {
                e / (n * (n - 1.0))
            } else {
                0.0
            };
            model.predict(n, e, density)
        }
        _ => PhysicsParams::default_params(),
    };

    let grav = params.gravitational_constant;
    let central = params.central_gravity;
    let spring_len = params.spring_length;
    let spring_k = params.spring_constant;
    let damp = params.damping;
    let overlap = params.avoid_overlap;

    format!(
        r##"<script>
const RAW_NODES = {nodes_json};
const RAW_EDGES = {edges_json};
const LEGEND = {legend_json};

const nodesDS = new vis.DataSet(RAW_NODES.map(n => ({{
  id: n.id, label: n.label, color: n.color, size: n.size,
  font: n.font, title: n.title,
  _community: n.community, _community_name: n.community_name,
  _source_file: n.source_file, _file_type: n.file_type, _degree: n.degree,
}})));

const edgesDS = new vis.DataSet(RAW_EDGES.map((e, i) => ({{
  id: i, from: e.from, to: e.to,
  label: '',
  title: e.title,
  dashes: e.dashes,
  width: e.width,
  color: e.color,
  arrows: {{ to: {{ enabled: true, scaleFactor: 0.5 }} }},
}})));

const container = document.getElementById('graph');
const network = new vis.Network(container, {{ nodes: nodesDS, edges: edgesDS }}, {{
  physics: {{
    enabled: true,
    solver: 'forceAtlas2Based',
    forceAtlas2Based: {{
      gravitationalConstant: {grav},
      centralGravity: {central},
      springLength: {spring_len},
      springConstant: {spring_k},
      damping: {damp},
      avoidOverlap: {overlap},
    }},
    stabilization: {{ iterations: 200, fit: true }},
  }},
  interaction: {{
    hover: true,
    tooltipDelay: 100,
    hideEdgesOnDrag: true,
    navigationButtons: false,
    keyboard: false,
  }},
  nodes: {{ shape: 'dot', borderWidth: 1.5 }},
  edges: {{ smooth: {{ type: 'continuous', roundness: 0.2 }}, selectionWidth: 3 }},
}});

network.once('stabilizationIterationsDone', () => {{
  network.setOptions({{ physics: {{ enabled: false }} }});
}});

function showInfo(nodeId) {{
  const n = nodesDS.get(nodeId);
  if (!n) return;
  const neighborIds = network.getConnectedNodes(nodeId);
  const neighborItems = neighborIds.map(nid => {{
    const nb = nodesDS.get(nid);
    const color = nb ? nb.color.background : '#555';
    return `<span class="neighbor-link" style="border-left-color:${{color}}" onclick="focusNode('${{nid}}')">${{nb ? nb.label : nid}}</span>`;
  }}).join('');
  document.getElementById('info-content').innerHTML = `
    <div class="field"><b>${{n.label}}</b></div>
    <div class="field">Type: ${{n._file_type || 'unknown'}}</div>
    <div class="field">Community: ${{n._community_name}}</div>
    <div class="field">Source: ${{n._source_file || '-'}}</div>
    <div class="field">Degree: ${{n._degree}}</div>
    ${{neighborIds.length ? `<div class="field" style="margin-top:8px;color:#aaa;font-size:11px">Neighbors (${{neighborIds.length}})</div><div id="neighbors-list">${{neighborItems}}</div>` : ''}}
  `;
}}

function focusNode(nodeId) {{
  network.focus(nodeId, {{ scale: 1.4, animation: true }});
  network.selectNodes([nodeId]);
  showInfo(nodeId);
}}

network.on('click', params => {{
  if (params.nodes.length > 0) showInfo(params.nodes[0]);
  else document.getElementById('info-content').innerHTML = '<span class="empty">Click a node to inspect it</span>';
}});

const searchInput = document.getElementById('search');
const searchResults = document.getElementById('search-results');
searchInput.addEventListener('input', () => {{
  const q = searchInput.value.toLowerCase().trim();
  searchResults.innerHTML = '';
  if (!q) {{ searchResults.style.display = 'none'; return; }}
  const matches = RAW_NODES.filter(n => n.label.toLowerCase().includes(q)).slice(0, 20);
  if (!matches.length) {{ searchResults.style.display = 'none'; return; }}
  searchResults.style.display = 'block';
  matches.forEach(n => {{
    const el = document.createElement('div');
    el.className = 'search-item';
    el.textContent = n.label;
    el.style.borderLeft = `3px solid ${{n.color.background}}`;
    el.style.paddingLeft = '8px';
    el.onclick = () => {{
      network.focus(n.id, {{ scale: 1.5, animation: true }});
      network.selectNodes([n.id]);
      showInfo(n.id);
      searchResults.style.display = 'none';
      searchInput.value = '';
    }};
    searchResults.appendChild(el);
  }});
}});
document.addEventListener('click', e => {{
  if (!searchResults.contains(e.target) && e.target !== searchInput)
    searchResults.style.display = 'none';
}});

const hiddenCommunities = new Set();
const legendEl = document.getElementById('legend');
LEGEND.forEach(c => {{
  const item = document.createElement('div');
  item.className = 'legend-item';
  item.innerHTML = `<div class="legend-dot" style="background:${{c.color}}"></div>
    <span class="legend-label">${{c.label}}</span>
    <span class="legend-count">${{c.count}}</span>`;
  item.onclick = () => {{
    if (hiddenCommunities.has(c.cid)) {{
      hiddenCommunities.delete(c.cid);
      item.classList.remove('dimmed');
    }} else {{
      hiddenCommunities.add(c.cid);
      item.classList.add('dimmed');
    }}
    const updates = RAW_NODES
      .filter(n => n.community === c.cid)
      .map(n => ({{ id: n.id, hidden: hiddenCommunities.has(c.cid) }}));
    nodesDS.update(updates);
  }};
  legendEl.appendChild(item);
}});
</script>"##,
        nodes_json = nodes_json,
        edges_json = edges_json,
        legend_json = legend_json,
    )
}

fn build_hyperedge_script(hyperedges_json: &str) -> String {
    format!(
        r##"<script>
const hyperedges = {hyperedges_json};
function drawHyperedges() {{
    const canvas = network.canvas.frame.canvas;
    const ctx = canvas.getContext('2d');
    hyperedges.forEach(h => {{
        const positions = h.nodes
            .map(nid => network.getPositions([nid])[nid])
            .filter(p => p !== undefined);
        if (positions.length < 2) return;
        ctx.save();
        ctx.globalAlpha = 0.12;
        ctx.fillStyle = '#6366f1';
        ctx.strokeStyle = '#6366f1';
        ctx.lineWidth = 2;
        ctx.beginPath();
        const scale = network.getScale();
        const offset = network.getViewPosition();
        const toCanvas = (p) => ({{
            x: (p.x - offset.x) * scale + canvas.width / 2,
            y: (p.y - offset.y) * scale + canvas.height / 2
        }});
        const pts = positions.map(toCanvas);
        const cx = pts.reduce((s, p) => s + p.x, 0) / pts.length;
        const cy = pts.reduce((s, p) => s + p.y, 0) / pts.length;
        const expanded = pts.map(p => ({{
            x: cx + (p.x - cx) * 1.15,
            y: cy + (p.y - cy) * 1.15
        }}));
        ctx.moveTo(expanded[0].x, expanded[0].y);
        expanded.slice(1).forEach(p => ctx.lineTo(p.x, p.y));
        ctx.closePath();
        ctx.fill();
        ctx.globalAlpha = 0.4;
        ctx.stroke();
        ctx.globalAlpha = 0.8;
        ctx.fillStyle = '#4f46e5';
        ctx.font = 'bold 11px sans-serif';
        ctx.textAlign = 'center';
        ctx.fillText(h.label, cx, cy - 5);
        ctx.restore();
    }});
}}
network.on('afterDrawing', drawHyperedges);
</script>"##,
        hyperedges_json = hyperedges_json,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Confidence, Entity, EntityId, Relationship};

    fn entity(id: &str, source_file: &str) -> Entity {
        Entity {
            id: EntityId::new(id),
            label: id.to_owned(),
            file_type: "code".to_owned(),
            source_file: source_file.to_owned(),
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
    fn to_html_creates_file() {
        let kg = KnowledgeGraph::new(
            vec![entity("a", "a.py"), entity("b", "b.py")],
            vec![rel("a", "b")],
            vec![],
        );
        let communities = HashMap::from([(0, vec![EntityId::new("a"), EntityId::new("b")])]);
        let labels = HashMap::from([(0, "main".to_owned())]);

        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("graph.html");
        to_html(&kg, &communities, &labels, &output).unwrap();
        assert!(output.exists());

        let content = std::fs::read_to_string(&output).unwrap();
        assert!(content.contains("vis-network"));
        assert!(content.contains("search"));
        assert!(content.contains("legend"));
        assert!(content.contains("forceAtlas2Based"));
    }

    #[test]
    fn to_html_rejects_too_large() {
        let entities: Vec<Entity> = (0..5001)
            .map(|i| entity(&format!("n{i}"), "f.py"))
            .collect();
        let kg = KnowledgeGraph::new(entities, vec![], vec![]);
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("graph.html");
        let result = to_html(&kg, &HashMap::new(), &HashMap::new(), &output);
        assert!(result.is_err());
    }
}
