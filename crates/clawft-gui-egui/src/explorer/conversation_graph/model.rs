//! In-memory graph model for the conversation graph view (ADR-067 D2).
//!
//! G1: fixture + types. G2: [`GraphModel::merge_head`]. G4: pinned rebuild hooks.

use serde_json::Value;
use std::collections::HashMap;

/// Node lifecycle tag mirrored from kernel `NodeState` (ADR-062).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeStateTag {
    Speculative,
    Frontier,
    Committed,
    Stale,
    Pruned,
}

impl NodeStateTag {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Speculative => "Speculative",
            Self::Frontier => "Frontier",
            Self::Committed => "Committed",
            Self::Stale => "Stale",
            Self::Pruned => "Pruned",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Speculative" | "speculative" => Some(Self::Speculative),
            "Frontier" | "frontier" => Some(Self::Frontier),
            "Committed" | "committed" => Some(Self::Committed),
            "Stale" | "stale" => Some(Self::Stale),
            "Pruned" | "pruned" => Some(Self::Pruned),
            _ => None,
        }
    }
}

/// Speaker role for shape encoding (ADR-067 D6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeRole {
    User,
    Assistant,
    Tool,
    Speaker,
}

impl NodeRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Tool => "tool",
            Self::Speaker => "speaker",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "assistant" | "asst" => Self::Assistant,
            "tool" => Self::Tool,
            "speaker" => Self::Speaker,
            _ => Self::User,
        }
    }
}

/// Classification vector (ADR-067 D5) — may be empty until P2.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Classification {
    pub intent: Option<String>,
    pub topic: Option<String>,
    pub emotion: Option<String>,
    pub goal: Option<String>,
}

/// One causal / crossref node on the canvas.
#[derive(Debug, Clone)]
pub struct GNode {
    pub id: String,
    pub chain_seq: Option<u64>,
    pub conv_id: String,
    pub role: NodeRole,
    pub text: String,
    pub state: NodeStateTag,
    pub ts_ms: u64,
    pub classification: Classification,
}

/// Typed edge (causal or crossref).
#[derive(Debug, Clone)]
pub struct GEdge {
    pub source: String,
    pub target: String,
    pub kind: String,
    pub weight: f32,
}

/// Shared model for live head + scrubbed history (ADR-067 D2).
#[derive(Debug, Clone, Default)]
pub struct GraphModel {
    pub conv_id: String,
    pub nodes: Vec<GNode>,
    pub edges: Vec<GEdge>,
}

impl GraphModel {
    /// G1 fixture: 2 user + 2 assistant turns, Follows chain, Speaker crossref.
    pub fn fixture() -> Self {
        let conv = "fixture-conv".to_string();
        let nodes = vec![
            GNode {
                id: "n1".into(),
                chain_seq: Some(1),
                conv_id: conv.clone(),
                role: NodeRole::User,
                text: "What is the frontier state?".into(),
                state: NodeStateTag::Committed,
                ts_ms: 1_000,
                classification: Classification {
                    intent: Some("question".into()),
                    topic: Some("graph-view".into()),
                    emotion: Some("curious".into()),
                    goal: Some("understand-ecc".into()),
                },
            },
            GNode {
                id: "n2".into(),
                chain_seq: Some(2),
                conv_id: conv.clone(),
                role: NodeRole::Assistant,
                text: "Frontier is the live wavefront on the causal graph.".into(),
                state: NodeStateTag::Committed,
                ts_ms: 2_000,
                classification: Classification {
                    intent: Some("statement".into()),
                    topic: Some("graph-view".into()),
                    emotion: None,
                    goal: Some("explain".into()),
                },
            },
            GNode {
                id: "n3".into(),
                chain_seq: Some(3),
                conv_id: conv.clone(),
                role: NodeRole::User,
                text: "Show me when a node goes Stale.".into(),
                state: NodeStateTag::Committed,
                ts_ms: 3_000,
                classification: Classification {
                    intent: Some("request".into()),
                    topic: Some("graph-view".into()),
                    ..Default::default()
                },
            },
            GNode {
                id: "n4".into(),
                chain_seq: Some(4),
                conv_id: conv.clone(),
                role: NodeRole::Assistant,
                text: "Prune flips Committed → Stale without a chain event.".into(),
                state: NodeStateTag::Frontier,
                ts_ms: 4_000,
                classification: Classification {
                    intent: Some("statement".into()),
                    topic: Some("graph-view".into()),
                    ..Default::default()
                },
            },
            // Speaker identity off the main lane (crossref target).
            GNode {
                id: "spk-alice".into(),
                chain_seq: None,
                conv_id: conv.clone(),
                role: NodeRole::Speaker,
                text: "alice".into(),
                state: NodeStateTag::Committed,
                ts_ms: 1_000,
                classification: Classification::default(),
            },
        ];
        let edges = vec![
            GEdge {
                source: "n1".into(),
                target: "n2".into(),
                kind: "Follows".into(),
                weight: 1.0,
            },
            GEdge {
                source: "n2".into(),
                target: "n3".into(),
                kind: "Follows".into(),
                weight: 1.0,
            },
            GEdge {
                source: "n3".into(),
                target: "n4".into(),
                kind: "Follows".into(),
                weight: 1.0,
            },
            GEdge {
                source: "n1".into(),
                target: "spk-alice".into(),
                kind: "Speaker".into(),
                weight: 0.5,
            },
        ];
        Self {
            conv_id: conv,
            nodes,
            edges,
        }
    }

    /// G2: merge a live head snapshot (JSON `{nodes, edges}` shape).
    ///
    /// Appends new ids; patches existing nodes' state/classification in place.
    pub fn merge_head(&mut self, payload: &Value) -> Result<(), String> {
        let obj = payload
            .as_object()
            .ok_or_else(|| "graph payload must be object".to_string())?;
        let nodes = obj
            .get("nodes")
            .and_then(Value::as_array)
            .ok_or_else(|| "missing nodes array".to_string())?;
        let edges = obj
            .get("edges")
            .and_then(Value::as_array)
            .ok_or_else(|| "missing edges array".to_string())?;

        let mut by_id: HashMap<String, usize> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id.clone(), i))
            .collect();

        for n in nodes {
            let g = parse_node(n)?;
            if let Some(&idx) = by_id.get(&g.id) {
                let dest = &mut self.nodes[idx];
                dest.state = g.state;
                dest.classification = g.classification;
                dest.text = g.text;
                dest.chain_seq = g.chain_seq;
                dest.ts_ms = g.ts_ms;
            } else {
                by_id.insert(g.id.clone(), self.nodes.len());
                self.nodes.push(g);
            }
        }

        // Rebuild edge list from head (topology small per-conv).
        self.edges.clear();
        for e in edges {
            self.edges.push(parse_edge(e)?);
        }
        Ok(())
    }

    /// G4 hook: replace model with graph-state-at-T (caller supplies snapshot).
    pub fn replace_with(&mut self, other: GraphModel) {
        *self = other;
    }
}

fn parse_node(v: &Value) -> Result<GNode, String> {
    let id = v
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| "node.id required".to_string())?
        .to_string();
    let role = NodeRole::parse(v.get("role").and_then(Value::as_str).unwrap_or("user"));
    let state = v
        .get("state")
        .and_then(Value::as_str)
        .and_then(NodeStateTag::parse)
        .or_else(|| {
            v.get("kind")
                .and_then(Value::as_str)
                .and_then(NodeStateTag::parse)
        })
        .unwrap_or(NodeStateTag::Committed);
    let class = v.get("classification").cloned().unwrap_or(Value::Null);
    Ok(GNode {
        id,
        chain_seq: v.get("chain_seq").and_then(Value::as_u64),
        conv_id: v
            .get("conv_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        role,
        text: v
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        state,
        ts_ms: v.get("ts_ms").and_then(Value::as_u64).unwrap_or(0),
        classification: Classification {
            intent: class
                .get("intent")
                .and_then(Value::as_str)
                .map(str::to_string),
            topic: class
                .get("topic")
                .and_then(Value::as_str)
                .map(str::to_string),
            emotion: class
                .get("emotion")
                .and_then(Value::as_str)
                .map(str::to_string),
            goal: class
                .get("goal")
                .and_then(Value::as_str)
                .map(str::to_string),
        },
    })
}

fn parse_edge(v: &Value) -> Result<GEdge, String> {
    let source = v
        .get("source")
        .and_then(Value::as_str)
        .ok_or_else(|| "edge.source required".to_string())?
        .to_string();
    let target = v
        .get("target")
        .and_then(Value::as_str)
        .ok_or_else(|| "edge.target required".to_string())?
        .to_string();
    Ok(GEdge {
        source,
        target,
        kind: v
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("Follows")
            .to_string(),
        weight: v.get("weight").and_then(Value::as_f64).unwrap_or(1.0) as f32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn fixture_has_follows_and_speaker() {
        let m = GraphModel::fixture();
        assert!(m.nodes.len() >= 4);
        assert!(m.edges.iter().any(|e| e.kind == "Follows"));
        assert!(m.edges.iter().any(|e| e.kind == "Speaker"));
        assert!(m.nodes.iter().any(|n| n.state == NodeStateTag::Frontier));
    }

    #[test]
    fn merge_head_appends_and_patches_state() {
        let mut m = GraphModel::fixture();
        let n_before = m.nodes.len();
        let payload = json!({
            "nodes": [
                {
                    "id": "n4",
                    "role": "assistant",
                    "state": "Committed",
                    "text": "now committed",
                    "ts_ms": 4000,
                    "chain_seq": 4,
                    "conv_id": "fixture-conv",
                    "classification": {}
                },
                {
                    "id": "n5",
                    "role": "user",
                    "state": "Frontier",
                    "text": "new turn",
                    "ts_ms": 5000,
                    "chain_seq": 5,
                    "conv_id": "fixture-conv",
                    "classification": {}
                }
            ],
            "edges": [
                {"source": "n4", "target": "n5", "kind": "Follows", "weight": 1.0}
            ]
        });
        m.merge_head(&payload).unwrap();
        assert_eq!(m.nodes.len(), n_before + 1);
        let n4 = m.nodes.iter().find(|n| n.id == "n4").unwrap();
        assert_eq!(n4.state, NodeStateTag::Committed);
        assert_eq!(n4.text, "now committed");
        assert!(m.nodes.iter().any(|n| n.id == "n5"));
        assert_eq!(m.edges.len(), 1);
    }
}
