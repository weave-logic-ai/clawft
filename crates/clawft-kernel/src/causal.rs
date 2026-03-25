//! Causal graph DAG with typed/weighted directed edges.
//!
//! Provides a concurrent, lock-free causal reasoning graph where nodes
//! represent events or observations and edges encode causal relationships
//! with weights and provenance metadata. Built on `DashMap` for safe
//! concurrent access from multiple agent threads.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

/// Numeric identifier for causal graph nodes.
///
/// This is local to the causal module and distinct from
/// [`crate::cluster::NodeId`] which is a `String`.
pub type NodeId = u64;

// ---------------------------------------------------------------------------
// CausalEdgeType
// ---------------------------------------------------------------------------

/// The kind of causal relationship an edge represents.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum CausalEdgeType {
    /// A directly causes B.
    Causes,
    /// A suppresses or prevents B.
    Inhibits,
    /// A and B are statistically correlated (non-directional semantics,
    /// but stored in the directed graph for traversal purposes).
    Correlates,
    /// A is a precondition that enables B.
    Enables,
    /// A temporally follows B.
    Follows,
    /// A provides evidence against B.
    Contradicts,
    /// Edge was created by a ClawStage trigger.
    TriggeredBy,
    /// A provides supporting evidence for B.
    EvidenceFor,
}

impl fmt::Display for CausalEdgeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Causes => write!(f, "Causes"),
            Self::Inhibits => write!(f, "Inhibits"),
            Self::Correlates => write!(f, "Correlates"),
            Self::Enables => write!(f, "Enables"),
            Self::Follows => write!(f, "Follows"),
            Self::Contradicts => write!(f, "Contradicts"),
            Self::TriggeredBy => write!(f, "TriggeredBy"),
            Self::EvidenceFor => write!(f, "EvidenceFor"),
        }
    }
}

// ---------------------------------------------------------------------------
// CausalEdge
// ---------------------------------------------------------------------------

/// A weighted, typed directed edge between two causal graph nodes.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CausalEdge {
    /// Source node (tail of the arrow).
    pub source: NodeId,
    /// Target node (head of the arrow).
    pub target: NodeId,
    /// Semantic type of the relationship.
    pub edge_type: CausalEdgeType,
    /// Strength / confidence of the relationship (0.0 .. 1.0 typical).
    pub weight: f32,
    /// Hybrid logical clock timestamp at creation.
    pub timestamp: u64,
    /// ExoChain sequence number for provenance tracking.
    pub chain_seq: u64,
    /// Universal Node ID bytes for the source node.
    pub source_universal_id: [u8; 32],
    /// Universal Node ID bytes for the target node.
    pub target_universal_id: [u8; 32],
}

// ---------------------------------------------------------------------------
// CausalNode
// ---------------------------------------------------------------------------

/// A node in the causal graph representing an event, observation, or concept.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CausalNode {
    /// Local numeric identifier.
    pub id: NodeId,
    /// Human-readable label.
    pub label: String,
    /// HLC timestamp at creation.
    pub created_at: u64,
    /// Arbitrary JSON metadata attached to this node.
    pub metadata: serde_json::Value,
}

// ---------------------------------------------------------------------------
// CausalGraph
// ---------------------------------------------------------------------------

/// A concurrent directed acyclic graph for causal reasoning.
///
/// Internally backed by [`DashMap`] for lock-free concurrent reads and
/// fine-grained write locking. Edge lists are stored in both forward
/// (outgoing) and reverse (incoming) adjacency maps for efficient
/// bidirectional traversal.
pub struct CausalGraph {
    nodes: DashMap<NodeId, CausalNode>,
    forward_edges: DashMap<NodeId, Vec<CausalEdge>>,
    reverse_edges: DashMap<NodeId, Vec<CausalEdge>>,
    next_node_id: AtomicU64,
    node_count: AtomicU64,
    edge_count: AtomicU64,
}

impl CausalGraph {
    /// Create an empty causal graph.
    pub fn new() -> Self {
        Self {
            nodes: DashMap::new(),
            forward_edges: DashMap::new(),
            reverse_edges: DashMap::new(),
            next_node_id: AtomicU64::new(1),
            node_count: AtomicU64::new(0),
            edge_count: AtomicU64::new(0),
        }
    }

    /// Add a node with an auto-assigned ID.
    pub fn add_node(&self, label: String, metadata: serde_json::Value) -> NodeId {
        let id = self.next_node_id.fetch_add(1, Ordering::SeqCst);
        let node = CausalNode {
            id,
            label,
            created_at: 0, // caller may set via metadata; HLC not available here
            metadata,
        };
        self.nodes.insert(id, node);
        self.forward_edges.insert(id, Vec::new());
        self.reverse_edges.insert(id, Vec::new());
        self.node_count.fetch_add(1, Ordering::SeqCst);
        id
    }

    /// Retrieve a clone of the node with the given ID.
    pub fn get_node(&self, id: NodeId) -> Option<CausalNode> {
        self.nodes.get(&id).map(|r| r.value().clone())
    }

    /// Remove a node and all edges incident to it.
    ///
    /// Returns the removed node, or `None` if the ID was not found.
    pub fn remove_node(&self, id: NodeId) -> Option<CausalNode> {
        let (_, node) = self.nodes.remove(&id)?;

        // Remove forward edges from this node and update reverse adjacency.
        if let Some((_, fwd)) = self.forward_edges.remove(&id) {
            let removed = fwd.len() as u64;
            for edge in &fwd {
                if let Some(mut rev) = self.reverse_edges.get_mut(&edge.target) {
                    rev.retain(|e| e.source != id);
                }
            }
            self.edge_count.fetch_sub(removed, Ordering::SeqCst);
        }

        // Remove reverse edges to this node and update forward adjacency.
        if let Some((_, rev)) = self.reverse_edges.remove(&id) {
            let removed = rev.len() as u64;
            for edge in &rev {
                if let Some(mut fwd) = self.forward_edges.get_mut(&edge.source) {
                    fwd.retain(|e| e.target != id);
                }
            }
            self.edge_count.fetch_sub(removed, Ordering::SeqCst);
        }

        self.node_count.fetch_sub(1, Ordering::SeqCst);
        Some(node)
    }

    /// Create an edge from `source` to `target`.
    ///
    /// Returns `false` if either endpoint does not exist.
    pub fn link(
        &self,
        source: NodeId,
        target: NodeId,
        edge_type: CausalEdgeType,
        weight: f32,
        timestamp: u64,
        chain_seq: u64,
    ) -> bool {
        if !self.nodes.contains_key(&source) || !self.nodes.contains_key(&target) {
            return false;
        }

        let edge = CausalEdge {
            source,
            target,
            edge_type,
            weight,
            timestamp,
            chain_seq,
            source_universal_id: [0u8; 32],
            target_universal_id: [0u8; 32],
        };

        if let Some(mut fwd) = self.forward_edges.get_mut(&source) {
            fwd.push(edge.clone());
        }
        if let Some(mut rev) = self.reverse_edges.get_mut(&target) {
            rev.push(edge);
        }

        self.edge_count.fetch_add(1, Ordering::SeqCst);
        true
    }

    /// Remove all edges between `source` and `target` (in that direction).
    ///
    /// Returns the number of edges removed.
    pub fn unlink(&self, source: NodeId, target: NodeId) -> usize {
        let mut count = 0usize;

        if let Some(mut fwd) = self.forward_edges.get_mut(&source) {
            let before = fwd.len();
            fwd.retain(|e| e.target != target);
            count = before - fwd.len();
        }

        if let Some(mut rev) = self.reverse_edges.get_mut(&target) {
            rev.retain(|e| e.source != source);
        }

        self.edge_count
            .fetch_sub(count as u64, Ordering::SeqCst);
        count
    }

    /// Return all edges originating from `id`.
    pub fn get_forward_edges(&self, id: NodeId) -> Vec<CausalEdge> {
        self.forward_edges
            .get(&id)
            .map(|r| r.value().clone())
            .unwrap_or_default()
    }

    /// Return all edges targeting `id`.
    pub fn get_reverse_edges(&self, id: NodeId) -> Vec<CausalEdge> {
        self.reverse_edges
            .get(&id)
            .map(|r| r.value().clone())
            .unwrap_or_default()
    }

    /// Return forward edges from `id` that match `edge_type`.
    pub fn get_edges_by_type(&self, id: NodeId, edge_type: &CausalEdgeType) -> Vec<CausalEdge> {
        self.forward_edges
            .get(&id)
            .map(|r| {
                r.value()
                    .iter()
                    .filter(|e| &e.edge_type == edge_type)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Number of nodes currently in the graph.
    pub fn node_count(&self) -> u64 {
        self.node_count.load(Ordering::SeqCst)
    }

    /// Number of edges currently in the graph.
    pub fn edge_count(&self) -> u64 {
        self.edge_count.load(Ordering::SeqCst)
    }

    /// Remove all nodes and edges (used during calibration cleanup).
    pub fn clear(&self) {
        self.nodes.clear();
        self.forward_edges.clear();
        self.reverse_edges.clear();
        self.node_count.store(0, Ordering::SeqCst);
        self.edge_count.store(0, Ordering::SeqCst);
        // Note: next_node_id is intentionally NOT reset so IDs remain unique
        // across clear cycles.
    }

    /// BFS traversal forward from `start` up to `depth` hops.
    ///
    /// Returns all discovered node IDs (excluding `start`).
    pub fn traverse_forward(&self, start: NodeId, depth: usize) -> Vec<NodeId> {
        self.bfs(start, depth, true)
    }

    /// BFS traversal backward (following reverse edges) from `start`
    /// up to `depth` hops.
    ///
    /// Returns all discovered node IDs (excluding `start`).
    pub fn traverse_reverse(&self, start: NodeId, depth: usize) -> Vec<NodeId> {
        self.bfs(start, depth, false)
    }

    /// Find the shortest path from `from` to `to` using BFS, limited
    /// to `max_depth` hops.
    ///
    /// Returns the node sequence including both endpoints, or `None`
    /// if no path exists within the depth limit.
    pub fn find_path(&self, from: NodeId, to: NodeId, max_depth: usize) -> Option<Vec<NodeId>> {
        if from == to {
            return Some(vec![from]);
        }
        if !self.nodes.contains_key(&from) || !self.nodes.contains_key(&to) {
            return None;
        }

        // BFS with parent tracking.
        let mut visited: std::collections::HashSet<NodeId> = std::collections::HashSet::new();
        let mut parent: std::collections::HashMap<NodeId, NodeId> =
            std::collections::HashMap::new();
        let mut queue: VecDeque<(NodeId, usize)> = VecDeque::new();

        visited.insert(from);
        queue.push_back((from, 0));

        while let Some((current, d)) = queue.pop_front() {
            if d >= max_depth {
                continue;
            }
            let edges = self.get_forward_edges(current);
            for edge in edges {
                if visited.contains(&edge.target) {
                    continue;
                }
                visited.insert(edge.target);
                parent.insert(edge.target, current);

                if edge.target == to {
                    // Reconstruct path.
                    let mut path = Vec::new();
                    let mut cur = to;
                    while cur != from {
                        path.push(cur);
                        cur = parent[&cur];
                    }
                    path.push(from);
                    path.reverse();
                    return Some(path);
                }

                queue.push_back((edge.target, d + 1));
            }
        }

        None
    }

    // -- private helpers --

    fn bfs(&self, start: NodeId, depth: usize, forward: bool) -> Vec<NodeId> {
        let mut visited: std::collections::HashSet<NodeId> = std::collections::HashSet::new();
        let mut result = Vec::new();
        let mut queue: VecDeque<(NodeId, usize)> = VecDeque::new();

        visited.insert(start);
        queue.push_back((start, 0));

        while let Some((current, d)) = queue.pop_front() {
            if d >= depth {
                continue;
            }
            let edges = if forward {
                self.get_forward_edges(current)
            } else {
                self.get_reverse_edges(current)
            };
            for edge in edges {
                let neighbor = if forward { edge.target } else { edge.source };
                if visited.contains(&neighbor) {
                    continue;
                }
                visited.insert(neighbor);
                result.push(neighbor);
                queue.push_back((neighbor, d + 1));
            }
        }

        result
    }
}

impl Default for CausalGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_graph() -> CausalGraph {
        CausalGraph::new()
    }

    // 1
    #[test]
    fn new_graph_empty() {
        let g = make_graph();
        assert_eq!(g.node_count(), 0);
        assert_eq!(g.edge_count(), 0);
    }

    // 2
    #[test]
    fn add_node_returns_id() {
        let g = make_graph();
        let id1 = g.add_node("A".into(), serde_json::json!({}));
        let id2 = g.add_node("B".into(), serde_json::json!({}));
        assert_ne!(id1, id2);
        assert_eq!(g.node_count(), 2);
    }

    // 3
    #[test]
    fn get_node() {
        let g = make_graph();
        let id = g.add_node("hello".into(), serde_json::json!({"key": "val"}));
        let node = g.get_node(id).unwrap();
        assert_eq!(node.label, "hello");
        assert_eq!(node.metadata["key"], "val");
    }

    // 4
    #[test]
    fn remove_node() {
        let g = make_graph();
        let id = g.add_node("X".into(), serde_json::json!({}));
        assert!(g.get_node(id).is_some());
        let removed = g.remove_node(id).unwrap();
        assert_eq!(removed.label, "X");
        assert!(g.get_node(id).is_none());
        assert_eq!(g.node_count(), 0);
    }

    // 5
    #[test]
    fn link_creates_edge() {
        let g = make_graph();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        assert!(g.link(a, b, CausalEdgeType::Causes, 0.9, 100, 1));
        assert_eq!(g.edge_count(), 1);
    }

    // 6
    #[test]
    fn link_invalid_source_returns_false() {
        let g = make_graph();
        let b = g.add_node("B".into(), serde_json::json!({}));
        assert!(!g.link(9999, b, CausalEdgeType::Causes, 0.5, 0, 0));
        assert_eq!(g.edge_count(), 0);
    }

    // 7
    #[test]
    fn link_invalid_target_returns_false() {
        let g = make_graph();
        let a = g.add_node("A".into(), serde_json::json!({}));
        assert!(!g.link(a, 9999, CausalEdgeType::Causes, 0.5, 0, 0));
        assert_eq!(g.edge_count(), 0);
    }

    // 8
    #[test]
    fn get_forward_edges() {
        let g = make_graph();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        let c = g.add_node("C".into(), serde_json::json!({}));
        g.link(a, b, CausalEdgeType::Causes, 1.0, 0, 0);
        g.link(a, c, CausalEdgeType::Enables, 0.5, 0, 0);
        let fwd = g.get_forward_edges(a);
        assert_eq!(fwd.len(), 2);
        assert!(fwd.iter().any(|e| e.target == b));
        assert!(fwd.iter().any(|e| e.target == c));
    }

    // 9
    #[test]
    fn get_reverse_edges() {
        let g = make_graph();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        g.link(a, b, CausalEdgeType::Causes, 1.0, 0, 0);
        let rev = g.get_reverse_edges(b);
        assert_eq!(rev.len(), 1);
        assert_eq!(rev[0].source, a);
    }

    // 10
    #[test]
    fn get_edges_by_type() {
        let g = make_graph();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        let c = g.add_node("C".into(), serde_json::json!({}));
        g.link(a, b, CausalEdgeType::Causes, 1.0, 0, 0);
        g.link(a, c, CausalEdgeType::Inhibits, 0.3, 0, 0);
        let causes = g.get_edges_by_type(a, &CausalEdgeType::Causes);
        assert_eq!(causes.len(), 1);
        assert_eq!(causes[0].target, b);
    }

    // 11
    #[test]
    fn unlink_removes_edges() {
        let g = make_graph();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        g.link(a, b, CausalEdgeType::Causes, 1.0, 0, 0);
        g.link(a, b, CausalEdgeType::Enables, 0.5, 0, 0);
        assert_eq!(g.edge_count(), 2);
        let removed = g.unlink(a, b);
        assert_eq!(removed, 2);
        assert_eq!(g.edge_count(), 0);
        assert!(g.get_forward_edges(a).is_empty());
        assert!(g.get_reverse_edges(b).is_empty());
    }

    // 12
    #[test]
    fn node_count() {
        let g = make_graph();
        assert_eq!(g.node_count(), 0);
        g.add_node("A".into(), serde_json::json!({}));
        assert_eq!(g.node_count(), 1);
        let id = g.add_node("B".into(), serde_json::json!({}));
        assert_eq!(g.node_count(), 2);
        g.remove_node(id);
        assert_eq!(g.node_count(), 1);
    }

    // 13
    #[test]
    fn edge_count() {
        let g = make_graph();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        let c = g.add_node("C".into(), serde_json::json!({}));
        g.link(a, b, CausalEdgeType::Causes, 1.0, 0, 0);
        g.link(b, c, CausalEdgeType::Follows, 0.8, 0, 0);
        assert_eq!(g.edge_count(), 2);
    }

    // 14
    #[test]
    fn clear_empties_graph() {
        let g = make_graph();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        g.link(a, b, CausalEdgeType::Causes, 1.0, 0, 0);
        g.clear();
        assert_eq!(g.node_count(), 0);
        assert_eq!(g.edge_count(), 0);
    }

    // 15 — A -> B, traverse 1 hop forward from A
    #[test]
    fn traverse_forward_single_hop() {
        let g = make_graph();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        g.link(a, b, CausalEdgeType::Causes, 1.0, 0, 0);
        let reachable = g.traverse_forward(a, 1);
        assert_eq!(reachable, vec![b]);
    }

    // 16 — A -> B -> C, traverse 2 hops from A
    #[test]
    fn traverse_forward_multi_hop() {
        let g = make_graph();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        let c = g.add_node("C".into(), serde_json::json!({}));
        g.link(a, b, CausalEdgeType::Causes, 1.0, 0, 0);
        g.link(b, c, CausalEdgeType::Causes, 1.0, 0, 0);
        let reachable = g.traverse_forward(a, 2);
        assert!(reachable.contains(&b));
        assert!(reachable.contains(&c));
        assert_eq!(reachable.len(), 2);
    }

    // 17 — A -> B -> C, traverse only 1 hop from A (should NOT reach C)
    #[test]
    fn traverse_forward_depth_limit() {
        let g = make_graph();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        let c = g.add_node("C".into(), serde_json::json!({}));
        g.link(a, b, CausalEdgeType::Causes, 1.0, 0, 0);
        g.link(b, c, CausalEdgeType::Causes, 1.0, 0, 0);
        let reachable = g.traverse_forward(a, 1);
        assert_eq!(reachable, vec![b]);
        assert!(!reachable.contains(&c));
    }

    // 18 — A -> B -> C, traverse reverse from C
    #[test]
    fn traverse_reverse() {
        let g = make_graph();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        let c = g.add_node("C".into(), serde_json::json!({}));
        g.link(a, b, CausalEdgeType::Causes, 1.0, 0, 0);
        g.link(b, c, CausalEdgeType::Causes, 1.0, 0, 0);
        let reachable = g.traverse_reverse(c, 2);
        assert!(reachable.contains(&b));
        assert!(reachable.contains(&a));
    }

    // 19
    #[test]
    fn find_path_exists() {
        let g = make_graph();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        let c = g.add_node("C".into(), serde_json::json!({}));
        g.link(a, b, CausalEdgeType::Causes, 1.0, 0, 0);
        g.link(b, c, CausalEdgeType::Causes, 1.0, 0, 0);
        let path = g.find_path(a, c, 5).unwrap();
        assert_eq!(path, vec![a, b, c]);
    }

    // 20
    #[test]
    fn find_path_no_path() {
        let g = make_graph();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        // No edge between them.
        assert!(g.find_path(a, b, 5).is_none());
    }

    // 21
    #[test]
    fn concurrent_add_nodes() {
        use std::sync::Arc;
        use std::thread;

        let g = Arc::new(CausalGraph::new());
        let mut handles = Vec::new();

        for t in 0..4 {
            let g = Arc::clone(&g);
            handles.push(thread::spawn(move || {
                for i in 0..25 {
                    g.add_node(format!("t{t}-n{i}"), serde_json::json!({}));
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(g.node_count(), 100);
    }

    // 22
    #[test]
    fn causal_edge_type_display() {
        assert_eq!(CausalEdgeType::Causes.to_string(), "Causes");
        assert_eq!(CausalEdgeType::Inhibits.to_string(), "Inhibits");
        assert_eq!(CausalEdgeType::Correlates.to_string(), "Correlates");
        assert_eq!(CausalEdgeType::Enables.to_string(), "Enables");
        assert_eq!(CausalEdgeType::Follows.to_string(), "Follows");
        assert_eq!(CausalEdgeType::Contradicts.to_string(), "Contradicts");
        assert_eq!(CausalEdgeType::TriggeredBy.to_string(), "TriggeredBy");
        assert_eq!(CausalEdgeType::EvidenceFor.to_string(), "EvidenceFor");
    }
}
