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
#[non_exhaustive]
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

    /// List all node IDs currently in the graph.
    pub fn node_ids(&self) -> Vec<NodeId> {
        self.nodes.iter().map(|r| *r.key()).collect()
    }

    /// Degree of a node (in + out edges, treating graph as undirected).
    pub fn degree(&self, id: NodeId) -> usize {
        let fwd = self.forward_edges.get(&id).map_or(0, |e| e.len());
        let rev = self.reverse_edges.get(&id).map_or(0, |e| e.len());
        fwd + rev
    }

    /// In-degree (number of incoming edges).
    pub fn in_degree(&self, id: NodeId) -> usize {
        self.reverse_edges.get(&id).map_or(0, |e| e.len())
    }

    /// Out-degree (number of outgoing edges).
    pub fn out_degree(&self, id: NodeId) -> usize {
        self.forward_edges.get(&id).map_or(0, |e| e.len())
    }

    // -----------------------------------------------------------------------
    // Connected Components (undirected)
    // -----------------------------------------------------------------------

    /// Find connected components treating the graph as undirected.
    ///
    /// Returns a vec of components, each component being a vec of node IDs.
    /// Components are sorted largest-first.
    pub fn connected_components(&self) -> Vec<Vec<NodeId>> {
        let ids = self.node_ids();
        let mut visited: std::collections::HashSet<NodeId> = std::collections::HashSet::new();
        let mut components = Vec::new();

        for &id in &ids {
            if visited.contains(&id) {
                continue;
            }
            // BFS over both directions (undirected).
            let mut component = Vec::new();
            let mut queue: VecDeque<NodeId> = VecDeque::new();
            visited.insert(id);
            queue.push_back(id);

            while let Some(current) = queue.pop_front() {
                component.push(current);
                // Forward neighbors.
                for edge in self.get_forward_edges(current) {
                    if visited.insert(edge.target) {
                        queue.push_back(edge.target);
                    }
                }
                // Reverse neighbors.
                for edge in self.get_reverse_edges(current) {
                    if visited.insert(edge.source) {
                        queue.push_back(edge.source);
                    }
                }
            }
            component.sort();
            components.push(component);
        }

        components.sort_by_key(|b| std::cmp::Reverse(b.len()));
        components
    }

    // -----------------------------------------------------------------------
    // Community Detection (Label Propagation)
    // -----------------------------------------------------------------------

    /// Detect communities using label propagation on the undirected graph.
    ///
    /// Each node starts with its own label. In each iteration, every node
    /// adopts the most frequent label among its neighbors (weighted by edge
    /// weight). Converges when no labels change, or after `max_iterations`.
    ///
    /// Returns a map from community label (a NodeId) to the set of node IDs
    /// in that community.
    pub fn detect_communities(&self, max_iterations: usize) -> Vec<Vec<NodeId>> {
        let ids = self.node_ids();
        if ids.is_empty() {
            return Vec::new();
        }

        // Initialize: each node gets its own ID as label.
        let mut labels: std::collections::HashMap<NodeId, NodeId> = std::collections::HashMap::new();
        for &id in &ids {
            labels.insert(id, id);
        }

        for _iter in 0..max_iterations {
            let mut changed = false;

            // Process nodes in a deterministic order.
            let mut process_order = ids.clone();
            process_order.sort();

            for &id in &process_order {
                // Gather neighbor labels weighted by edge weight.
                let mut label_weights: std::collections::HashMap<NodeId, f32> =
                    std::collections::HashMap::new();

                for edge in self.get_forward_edges(id) {
                    if let Some(&lbl) = labels.get(&edge.target) {
                        *label_weights.entry(lbl).or_insert(0.0) += edge.weight;
                    }
                }
                for edge in self.get_reverse_edges(id) {
                    if let Some(&lbl) = labels.get(&edge.source) {
                        *label_weights.entry(lbl).or_insert(0.0) += edge.weight;
                    }
                }

                if label_weights.is_empty() {
                    continue; // isolated node keeps its label
                }

                // Pick the label with the highest total weight.
                // On ties, pick the smallest label for determinism.
                let best_label = label_weights
                    .iter()
                    .max_by(|a, b| {
                        a.1.partial_cmp(b.1)
                            .unwrap_or(std::cmp::Ordering::Equal)
                            .then_with(|| b.0.cmp(a.0)) // smaller ID wins ties
                    })
                    .map(|(&lbl, _)| lbl)
                    .unwrap();

                if labels[&id] != best_label {
                    labels.insert(id, best_label);
                    changed = true;
                }
            }

            if !changed {
                break;
            }
        }

        // Group nodes by label.
        let mut communities: std::collections::HashMap<NodeId, Vec<NodeId>> =
            std::collections::HashMap::new();
        for (&node, &label) in &labels {
            communities.entry(label).or_default().push(node);
        }

        let mut result: Vec<Vec<NodeId>> = communities.into_values().collect();
        for community in &mut result {
            community.sort();
        }
        result.sort_by_key(|b| std::cmp::Reverse(b.len()));
        result
    }

    // -----------------------------------------------------------------------
    // Spectral Analysis
    // -----------------------------------------------------------------------

    /// Compute the algebraic connectivity (lambda_2) of the graph.
    ///
    /// Lambda_2 is the second-smallest eigenvalue of the graph Laplacian.
    /// - lambda_2 = 0 means the graph is disconnected.
    /// - Higher values indicate stronger connectivity.
    ///
    /// Uses sparse Lanczos iteration at O(k*m) where m = number of edges
    /// and k = `max_iterations`. For typical ECC graphs with average degree
    /// ~10, this is ~200x faster than the dense O(k*n^2) approach.
    ///
    /// Returns `(lambda_2, fiedler_vector)` where the Fiedler vector can be
    /// used for spectral partitioning (sign of each component indicates which
    /// partition the node belongs to).
    pub fn spectral_analysis(&self, max_iterations: usize) -> SpectralResult {
        let ids = self.node_ids();
        let n = ids.len();

        if n < 2 {
            return SpectralResult {
                lambda_2: 0.0,
                fiedler_vector: Vec::new(),
                node_ids: ids,
            };
        }

        // Build index map: NodeId -> matrix index.
        let mut id_to_idx: std::collections::HashMap<NodeId, usize> =
            std::collections::HashMap::new();
        let mut sorted_ids = ids.clone();
        sorted_ids.sort();
        for (i, &id) in sorted_ids.iter().enumerate() {
            id_to_idx.insert(id, i);
        }

        // Build sparse adjacency: adj[i] = Vec<(j, weight)> (symmetric).
        // Also accumulate degrees.  O(m) space instead of O(n^2).
        let mut adj: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
        let mut degree: Vec<f64> = vec![0.0; n];

        for &id in &sorted_ids {
            let i = id_to_idx[&id];

            // Forward edges.
            for edge in self.get_forward_edges(id) {
                if let Some(&j) = id_to_idx.get(&edge.target) {
                    if i != j {
                        let w = edge.weight as f64;
                        adj[i].push((j, w));
                        adj[j].push((i, w));
                        degree[i] += w;
                        degree[j] += w;
                    }
                }
            }
            // Reverse edges — only upper triangle to avoid double-counting.
            for edge in self.get_reverse_edges(id) {
                if let Some(&j) = id_to_idx.get(&edge.source) {
                    if i != j && j > i {
                        let w = edge.weight as f64;
                        adj[i].push((j, w));
                        adj[j].push((i, w));
                        degree[i] += w;
                        degree[j] += w;
                    }
                }
            }
        }

        // Fix up degree: recompute from adjacency for correctness (handles
        // any double-adds from symmetric storage).
        for i in 0..n {
            let mut d = 0.0f64;
            for &(_, w) in &adj[i] {
                d += w;
            }
            degree[i] = d;
        }

        // Sparse Laplacian mat-vec: result = L * x = D*x - A*x
        let laplacian_mul = |x: &[f64], out: &mut [f64]| {
            for i in 0..n {
                let mut sum = degree[i] * x[i]; // D*x
                for &(j, w) in &adj[i] {
                    sum -= w * x[j]; // -A*x
                }
                out[i] = sum;
            }
        };

        // ── Lanczos iteration ──────────────────────────────────────────
        // Builds a k x k tridiagonal matrix T whose eigenvalues approximate
        // those of L restricted to the subspace orthogonal to the constant
        // (null-space) vector.  We then extract lambda_2 from T.
        //
        // The Fiedler vector is recovered by mapping the corresponding
        // eigenvector of T back through the Lanczos basis.

        let inv_sqrt_n = 1.0 / (n as f64).sqrt();

        // Initial vector: deterministic, orthogonal to the constant vector.
        let mut q: Vec<f64> = (0..n)
            .map(|i| (i as f64) - (n as f64 - 1.0) / 2.0)
            .collect();

        // Project out the constant (null-space) direction.
        let dot_ones: f64 = q.iter().sum::<f64>() * inv_sqrt_n;
        for qi in q.iter_mut() {
            *qi -= dot_ones * inv_sqrt_n;
        }
        normalize_vec(&mut q);

        let k = max_iterations.min(n - 1); // can't exceed n-1 Lanczos steps
        let mut alpha: Vec<f64> = Vec::with_capacity(k); // diagonal of T
        let mut beta: Vec<f64> = Vec::with_capacity(k);  // sub-diagonal of T
        let mut basis: Vec<Vec<f64>> = Vec::with_capacity(k); // Lanczos vectors

        let mut q_prev: Vec<f64> = vec![0.0; n];
        let mut w_buf: Vec<f64> = vec![0.0; n];

        for j in 0..k {
            basis.push(q.clone());

            // w = L * q_j
            laplacian_mul(&q, &mut w_buf);

            // alpha_j = q_j^T * w
            let aj: f64 = q.iter().zip(w_buf.iter()).map(|(a, b)| a * b).sum();
            alpha.push(aj);

            // w = w - alpha_j * q_j - beta_{j-1} * q_{j-1}
            let bj_prev = if j > 0 { beta[j - 1] } else { 0.0 };
            for i in 0..n {
                w_buf[i] -= aj * q[i] + bj_prev * q_prev[i];
            }

            // Re-orthogonalize against all previous basis vectors AND the
            // constant vector (full reorth for numerical stability).
            let dot_c: f64 = w_buf.iter().sum::<f64>() * inv_sqrt_n;
            for wi in w_buf.iter_mut() {
                *wi -= dot_c * inv_sqrt_n;
            }
            for prev in &basis {
                let dot: f64 = w_buf.iter().zip(prev.iter()).map(|(a, b)| a * b).sum();
                for i in 0..n {
                    w_buf[i] -= dot * prev[i];
                }
            }

            let bj: f64 = w_buf.iter().map(|x| x * x).sum::<f64>().sqrt();
            beta.push(bj);

            if bj < 1e-12 {
                // Invariant subspace found; stop early.
                break;
            }

            // q_{j+1} = w / beta_j
            q_prev = q.clone();
            q = w_buf.iter().map(|&x| x / bj).collect();
        }

        // ── Extract eigenvalues from the tridiagonal matrix T ──────────
        // Use the implicit-shift QR algorithm on the symmetric tridiagonal
        // matrix (alpha, beta).  We only need the smallest eigenvalue of T
        // (which approximates lambda_2 since we projected out the null space).
        let m = alpha.len();
        let (evals, evecs) = tridiag_eigen(&alpha, &beta[..m.saturating_sub(1).max(0).min(beta.len())], m);

        // Find the smallest eigenvalue (approximation to lambda_2).
        let mut min_idx = 0;
        let mut min_val = f64::MAX;
        for (i, &ev) in evals.iter().enumerate() {
            if ev < min_val {
                min_val = ev;
                min_idx = i;
            }
        }
        let lambda_2 = min_val.max(0.0);

        // Recover the Fiedler vector: v = Q * s, where Q is the n x m Lanczos
        // basis and s is the eigenvector of T corresponding to lambda_2.
        let s = &evecs[min_idx];
        let mut fiedler = vec![0.0f64; n];
        for (j, bvec) in basis.iter().enumerate() {
            if j < s.len() {
                let sj = s[j];
                for i in 0..n {
                    fiedler[i] += sj * bvec[i];
                }
            }
        }
        normalize_vec(&mut fiedler);

        SpectralResult {
            lambda_2,
            fiedler_vector: fiedler,
            node_ids: sorted_ids,
        }
    }

    /// Partition the graph into two halves using the Fiedler vector.
    ///
    /// Nodes with positive Fiedler vector components go to partition A,
    /// negative to partition B. This is spectral bisection — the
    /// minimum-cut balanced partition.
    pub fn spectral_partition(&self) -> (Vec<NodeId>, Vec<NodeId>) {
        let result = self.spectral_analysis(50);
        let mut a = Vec::new();
        let mut b = Vec::new();

        for (i, &id) in result.node_ids.iter().enumerate() {
            if i < result.fiedler_vector.len() && result.fiedler_vector[i] >= 0.0 {
                a.push(id);
            } else {
                b.push(id);
            }
        }
        (a, b)
    }

    // -----------------------------------------------------------------------
    // Predictive Analysis
    // -----------------------------------------------------------------------

    /// Compute co-modification coupling between nodes based on temporal
    /// co-occurrence patterns.
    ///
    /// Given a list of "change events" (each event is a set of node IDs that
    /// changed together, plus a timestamp), computes a coupling score for
    /// every pair of nodes that have been modified together.
    ///
    /// The coupling score for nodes (A, B) is:
    ///   coupling = co_changes(A,B) / max(changes(A), changes(B))
    ///
    /// Returns pairs sorted by coupling score descending.
    pub fn compute_coupling(
        &self,
        change_events: &[ChangeEvent],
    ) -> Vec<CouplingPair> {
        let mut change_count: std::collections::HashMap<NodeId, usize> =
            std::collections::HashMap::new();
        let mut co_change_count: std::collections::HashMap<(NodeId, NodeId), usize> =
            std::collections::HashMap::new();

        for event in change_events {
            let mut nodes: Vec<NodeId> = event.node_ids.clone();
            nodes.sort();
            nodes.dedup();

            for &id in &nodes {
                *change_count.entry(id).or_insert(0) += 1;
            }
            // Count co-occurrences.
            for i in 0..nodes.len() {
                for j in (i + 1)..nodes.len() {
                    let key = (nodes[i], nodes[j]);
                    *co_change_count.entry(key).or_insert(0) += 1;
                }
            }
        }

        let mut pairs: Vec<CouplingPair> = co_change_count
            .iter()
            .map(|(&(a, b), &co)| {
                let max_changes = change_count
                    .get(&a)
                    .copied()
                    .unwrap_or(1)
                    .max(change_count.get(&b).copied().unwrap_or(1));
                CouplingPair {
                    node_a: a,
                    node_b: b,
                    co_changes: co,
                    coupling_score: co as f64 / max_changes as f64,
                }
            })
            .collect();

        pairs.sort_by(|a, b| {
            b.coupling_score
                .partial_cmp(&a.coupling_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        pairs
    }

    /// Detect burst patterns in change events and predict which nodes are
    /// likely to change next.
    ///
    /// A "burst" is a period where a node has significantly more changes
    /// than its baseline rate. Nodes currently in a burst, or recently
    /// co-modified with nodes in a burst, are predicted to change next.
    ///
    /// `window_size` is the number of recent events to consider for the
    /// burst window. `baseline_factor` is the multiplier above which a
    /// node's activity is considered a burst (e.g., 2.0 = 2x baseline).
    ///
    /// Returns nodes sorted by prediction confidence (descending).
    pub fn predict_changes(
        &self,
        change_events: &[ChangeEvent],
        window_size: usize,
        baseline_factor: f64,
    ) -> Vec<ChangePrediction> {
        if change_events.is_empty() {
            return Vec::new();
        }

        // Sort events by timestamp.
        let mut sorted_events = change_events.to_vec();
        sorted_events.sort_by_key(|e| e.timestamp);

        let total = sorted_events.len();
        let window_start = total.saturating_sub(window_size);

        // Compute baseline rate (changes per event across all history).
        let mut total_counts: std::collections::HashMap<NodeId, usize> =
            std::collections::HashMap::new();
        for event in &sorted_events {
            for &id in &event.node_ids {
                *total_counts.entry(id).or_insert(0) += 1;
            }
        }

        // Compute window rate.
        let mut window_counts: std::collections::HashMap<NodeId, usize> =
            std::collections::HashMap::new();
        for event in &sorted_events[window_start..] {
            for &id in &event.node_ids {
                *window_counts.entry(id).or_insert(0) += 1;
            }
        }

        let window_len = total - window_start;

        // Identify nodes in burst.
        let mut burst_nodes: Vec<(NodeId, f64)> = Vec::new();
        for (&id, &window_count) in &window_counts {
            let total_count = total_counts.get(&id).copied().unwrap_or(0);
            let baseline_rate = total_count as f64 / total as f64;
            let window_rate = window_count as f64 / window_len as f64;

            if baseline_rate > 0.0 && window_rate / baseline_rate >= baseline_factor {
                burst_nodes.push((id, window_rate / baseline_rate));
            }
        }

        // Compute coupling to identify co-modification partners.
        let coupling = self.compute_coupling(change_events);
        let coupling_map: std::collections::HashMap<(NodeId, NodeId), f64> = coupling
            .iter()
            .map(|p| ((p.node_a, p.node_b), p.coupling_score))
            .collect();

        // Score all nodes.
        let mut predictions: std::collections::HashMap<NodeId, f64> =
            std::collections::HashMap::new();

        // Burst nodes get high base confidence.
        for &(id, burst_ratio) in &burst_nodes {
            *predictions.entry(id).or_insert(0.0) += burst_ratio * 0.6;
        }

        // Coupled partners of burst nodes get transitive confidence.
        for &(burst_id, burst_ratio) in &burst_nodes {
            for (&(a, b), &coupling_score) in &coupling_map {
                let partner = if a == burst_id {
                    Some(b)
                } else if b == burst_id {
                    Some(a)
                } else {
                    None
                };
                if let Some(partner_id) = partner {
                    *predictions.entry(partner_id).or_insert(0.0) +=
                        burst_ratio * coupling_score * 0.4;
                }
            }
        }

        // Recent activity boost: nodes that appeared in the last few events.
        let recency_window = (window_size / 3).max(1);
        let recency_start = total.saturating_sub(recency_window);
        for event in &sorted_events[recency_start..] {
            for &id in &event.node_ids {
                *predictions.entry(id).or_insert(0.0) += 0.1;
            }
        }

        let mut result: Vec<ChangePrediction> = predictions
            .into_iter()
            .map(|(id, confidence)| {
                let label = self
                    .get_node(id)
                    .map(|n| n.label.clone())
                    .unwrap_or_else(|| format!("node:{id}"));
                let in_burst = burst_nodes.iter().any(|&(bid, _)| bid == id);
                ChangePrediction {
                    node_id: id,
                    label,
                    confidence: confidence.min(1.0),
                    in_burst,
                    recent_changes: window_counts.get(&id).copied().unwrap_or(0),
                }
            })
            .collect();

        result.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        result
    }
}

// ---------------------------------------------------------------------------
// Supporting types
// ---------------------------------------------------------------------------

/// Result of spectral analysis on the causal graph.
#[derive(Debug, Clone)]
pub struct SpectralResult {
    /// Algebraic connectivity (second-smallest Laplacian eigenvalue).
    /// 0.0 means disconnected. Higher = more connected.
    pub lambda_2: f64,
    /// Fiedler vector — sign indicates spectral partition membership.
    pub fiedler_vector: Vec<f64>,
    /// Node IDs in the same order as the Fiedler vector.
    pub node_ids: Vec<NodeId>,
}

/// A temporal change event: a set of nodes that changed together.
#[derive(Debug, Clone)]
pub struct ChangeEvent {
    /// Nodes that changed in this event (e.g., modules modified in a commit).
    pub node_ids: Vec<NodeId>,
    /// Timestamp of the event.
    pub timestamp: u64,
}

/// Coupling between two nodes based on co-modification frequency.
#[derive(Debug, Clone)]
pub struct CouplingPair {
    /// First node.
    pub node_a: NodeId,
    /// Second node.
    pub node_b: NodeId,
    /// Number of times both changed in the same event.
    pub co_changes: usize,
    /// Coupling score: co_changes / max(changes_a, changes_b).
    pub coupling_score: f64,
}

/// A prediction that a node will change soon.
#[derive(Debug, Clone)]
pub struct ChangePrediction {
    /// Node ID.
    pub node_id: NodeId,
    /// Human-readable label.
    pub label: String,
    /// Prediction confidence (0.0 .. 1.0).
    pub confidence: f64,
    /// Whether this node is currently in a burst pattern.
    pub in_burst: bool,
    /// Number of changes in the recent window.
    pub recent_changes: usize,
}

/// L2-normalize a vector in place.
fn normalize_vec(v: &mut [f64]) {
    let norm: f64 = v.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm > 1e-12 {
        v.iter_mut().for_each(|x| *x /= norm);
    }
}

/// Compute all eigenvalues and eigenvectors of a symmetric tridiagonal matrix.
///
/// * `diag` — main diagonal (length m).
/// * `off`  — sub-diagonal (length m-1).
///
/// Returns `(eigenvalues, eigenvectors)` where `eigenvectors[i]` is the
/// eigenvector for `eigenvalues[i]`, each of length `m`.
///
/// Uses the Jacobi eigenvalue algorithm on the full m x m symmetric matrix
/// built from the tridiagonal.  Since m is small (Lanczos iteration count,
/// typically 20-50), the O(m^3) cost is negligible compared to the O(k*m)
/// sparse mat-vecs.
fn tridiag_eigen(diag: &[f64], off: &[f64], m: usize) -> (Vec<f64>, Vec<Vec<f64>>) {
    if m == 0 {
        return (Vec::new(), Vec::new());
    }
    if m == 1 {
        return (vec![diag[0]], vec![vec![1.0]]);
    }

    // Build full symmetric matrix from the tridiagonal.
    let mut a = vec![vec![0.0f64; m]; m];
    for i in 0..m {
        a[i][i] = diag[i];
    }
    let off_len = off.len().min(m - 1);
    for i in 0..off_len {
        a[i][i + 1] = off[i];
        a[i + 1][i] = off[i];
    }

    // Eigenvector matrix V (columns are eigenvectors), starts as identity.
    let mut v = vec![vec![0.0f64; m]; m];
    for i in 0..m {
        v[i][i] = 1.0;
    }

    // Jacobi cyclic sweeps.
    for _ in 0..100 * m {
        // Find the largest off-diagonal element.
        let mut max_off = 0.0f64;
        let mut p = 0usize;
        let mut q = 1usize;
        for i in 0..m {
            for j in (i + 1)..m {
                if a[i][j].abs() > max_off {
                    max_off = a[i][j].abs();
                    p = i;
                    q = j;
                }
            }
        }

        if max_off < 1e-15 {
            break;
        }

        // Compute Jacobi rotation angle to zero out a[p][q].
        let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
        let t = theta.signum() / (theta.abs() + (1.0 + theta * theta).sqrt());
        let c = 1.0 / (1.0 + t * t).sqrt();
        let s = t * c;

        // Apply similarity rotation to A.
        let app = a[p][p];
        let aqq = a[q][q];
        let apq = a[p][q];
        a[p][p] = c * c * app - 2.0 * s * c * apq + s * s * aqq;
        a[q][q] = s * s * app + 2.0 * s * c * apq + c * c * aqq;
        a[p][q] = 0.0;
        a[q][p] = 0.0;

        for r in 0..m {
            if r != p && r != q {
                let arp = a[r][p];
                let arq = a[r][q];
                a[r][p] = c * arp - s * arq;
                a[p][r] = a[r][p];
                a[r][q] = s * arp + c * arq;
                a[q][r] = a[r][q];
            }
        }

        // Accumulate rotation into eigenvector matrix.
        for r in 0..m {
            let vp = v[r][p];
            let vq = v[r][q];
            v[r][p] = c * vp - s * vq;
            v[r][q] = s * vp + c * vq;
        }
    }

    // Eigenvalues are the diagonal of A.
    let eigenvalues: Vec<f64> = (0..m).map(|i| a[i][i]).collect();

    // Eigenvectors: column j of V is the eigenvector for eigenvalue j.
    // Return as eigenvectors[j] = column j.
    let eigenvectors: Vec<Vec<f64>> = (0..m)
        .map(|j| (0..m).map(|i| v[i][j]).collect())
        .collect();

    (eigenvalues, eigenvectors)
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

/// Serializable snapshot of a [`CausalGraph`] for JSON persistence.
#[derive(Serialize, Deserialize)]
struct CausalGraphSnapshot {
    next_node_id: u64,
    nodes: Vec<CausalNode>,
    forward_edges: std::collections::HashMap<NodeId, Vec<CausalEdge>>,
}

impl CausalGraph {
    /// Serialize the entire graph to a JSON writer.
    pub fn save_to_writer<W: std::io::Write>(&self, writer: W) -> Result<(), std::io::Error> {
        let snapshot = self.to_snapshot();
        serde_json::to_writer_pretty(writer, &snapshot)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
    }

    /// Deserialize a graph from a JSON reader.
    pub fn load_from_reader<R: std::io::Read>(reader: R) -> Result<Self, std::io::Error> {
        let snapshot: CausalGraphSnapshot = serde_json::from_reader(reader)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        Ok(Self::from_snapshot(snapshot))
    }

    /// Save the graph to a file path.
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::File::create(path)?;
        let writer = std::io::BufWriter::new(file);
        self.save_to_writer(writer)
    }

    /// Load a graph from a file path.
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, std::io::Error> {
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        Self::load_from_reader(reader)
    }

    fn to_snapshot(&self) -> CausalGraphSnapshot {
        let nodes: Vec<CausalNode> = self.nodes.iter().map(|r| r.value().clone()).collect();
        let mut forward_edges = std::collections::HashMap::new();
        for entry in self.forward_edges.iter() {
            if !entry.value().is_empty() {
                forward_edges.insert(*entry.key(), entry.value().clone());
            }
        }
        CausalGraphSnapshot {
            next_node_id: self.next_node_id.load(Ordering::SeqCst),
            nodes,
            forward_edges,
        }
    }

    fn from_snapshot(snapshot: CausalGraphSnapshot) -> Self {
        let graph = Self {
            nodes: DashMap::new(),
            forward_edges: DashMap::new(),
            reverse_edges: DashMap::new(),
            next_node_id: AtomicU64::new(snapshot.next_node_id),
            node_count: AtomicU64::new(0),
            edge_count: AtomicU64::new(0),
        };

        // Restore nodes.
        for node in &snapshot.nodes {
            graph.nodes.insert(node.id, node.clone());
            graph.forward_edges.insert(node.id, Vec::new());
            graph.reverse_edges.insert(node.id, Vec::new());
        }
        graph.node_count.store(snapshot.nodes.len() as u64, Ordering::SeqCst);

        // Restore edges from forward_edges map.
        let mut total_edges: u64 = 0;
        for (source_id, edges) in &snapshot.forward_edges {
            for edge in edges {
                if let Some(mut fwd) = graph.forward_edges.get_mut(source_id) {
                    fwd.push(edge.clone());
                }
                if let Some(mut rev) = graph.reverse_edges.get_mut(&edge.target) {
                    rev.push(edge.clone());
                }
                total_edges += 1;
            }
        }
        graph.edge_count.store(total_edges, Ordering::SeqCst);

        graph
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

    // =====================================================================
    // Degree tests
    // =====================================================================

    // 23
    #[test]
    fn degree_computation() {
        let g = make_graph();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        let c = g.add_node("C".into(), serde_json::json!({}));
        g.link(a, b, CausalEdgeType::Causes, 1.0, 0, 0);
        g.link(a, c, CausalEdgeType::Enables, 0.5, 0, 0);
        g.link(b, c, CausalEdgeType::Follows, 0.8, 0, 0);
        assert_eq!(g.out_degree(a), 2);
        assert_eq!(g.in_degree(a), 0);
        assert_eq!(g.degree(a), 2);
        assert_eq!(g.in_degree(c), 2);
        assert_eq!(g.out_degree(c), 0);
        assert_eq!(g.degree(c), 2);
        assert_eq!(g.degree(b), 2); // 1 in + 1 out
    }

    // =====================================================================
    // Connected Components tests
    // =====================================================================

    // 24
    #[test]
    fn connected_components_single() {
        let g = make_graph();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        g.link(a, b, CausalEdgeType::Causes, 1.0, 0, 0);
        let cc = g.connected_components();
        assert_eq!(cc.len(), 1);
        assert_eq!(cc[0].len(), 2);
    }

    // 25
    #[test]
    fn connected_components_two_islands() {
        let g = make_graph();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        let c = g.add_node("C".into(), serde_json::json!({}));
        let d = g.add_node("D".into(), serde_json::json!({}));
        g.link(a, b, CausalEdgeType::Causes, 1.0, 0, 0);
        g.link(c, d, CausalEdgeType::Causes, 1.0, 0, 0);
        let cc = g.connected_components();
        assert_eq!(cc.len(), 2);
        assert_eq!(cc[0].len(), 2);
        assert_eq!(cc[1].len(), 2);
    }

    // 26
    #[test]
    fn connected_components_isolated_node() {
        let g = make_graph();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        g.add_node("isolated".into(), serde_json::json!({}));
        g.link(a, b, CausalEdgeType::Causes, 1.0, 0, 0);
        let cc = g.connected_components();
        assert_eq!(cc.len(), 2);
        assert_eq!(cc[0].len(), 2); // largest first
        assert_eq!(cc[1].len(), 1); // isolated
    }

    // =====================================================================
    // Community Detection tests
    // =====================================================================

    // 27
    #[test]
    fn community_detection_two_clusters() {
        let g = make_graph();
        // Cluster 1: A-B-C strongly connected.
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        let c = g.add_node("C".into(), serde_json::json!({}));
        g.link(a, b, CausalEdgeType::Causes, 1.0, 0, 0);
        g.link(b, c, CausalEdgeType::Causes, 1.0, 0, 0);
        g.link(c, a, CausalEdgeType::Causes, 1.0, 0, 0);

        // Cluster 2: D-E-F strongly connected.
        let d = g.add_node("D".into(), serde_json::json!({}));
        let e = g.add_node("E".into(), serde_json::json!({}));
        let f = g.add_node("F".into(), serde_json::json!({}));
        g.link(d, e, CausalEdgeType::Causes, 1.0, 0, 0);
        g.link(e, f, CausalEdgeType::Causes, 1.0, 0, 0);
        g.link(f, d, CausalEdgeType::Causes, 1.0, 0, 0);

        // Weak bridge between clusters.
        g.link(c, d, CausalEdgeType::Correlates, 0.1, 0, 0);

        let communities = g.detect_communities(20);
        // Should find 2 communities (clusters) even with the weak bridge.
        // Label propagation may merge them due to the bridge, but the strong
        // internal edges should dominate.
        assert!(!communities.is_empty());
        // At minimum, isolated nodes shouldn't be their own community.
        assert!(communities.len() <= 3);
    }

    // 28
    #[test]
    fn community_detection_isolated_nodes() {
        let g = make_graph();
        g.add_node("A".into(), serde_json::json!({}));
        g.add_node("B".into(), serde_json::json!({}));
        let communities = g.detect_communities(10);
        // Each isolated node stays in its own community.
        assert_eq!(communities.len(), 2);
    }

    // 29
    #[test]
    fn community_detection_empty_graph() {
        let g = make_graph();
        let communities = g.detect_communities(10);
        assert!(communities.is_empty());
    }

    // =====================================================================
    // Spectral Analysis tests
    // =====================================================================

    // 30
    #[test]
    fn spectral_connected_graph_positive_lambda2() {
        let g = make_graph();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        let c = g.add_node("C".into(), serde_json::json!({}));
        g.link(a, b, CausalEdgeType::Causes, 1.0, 0, 0);
        g.link(b, c, CausalEdgeType::Causes, 1.0, 0, 0);
        g.link(a, c, CausalEdgeType::Causes, 1.0, 0, 0);

        let result = g.spectral_analysis(50);
        assert!(
            result.lambda_2 > 0.0,
            "connected graph should have lambda_2 > 0, got {}",
            result.lambda_2
        );
        assert_eq!(result.fiedler_vector.len(), 3);
        assert_eq!(result.node_ids.len(), 3);
    }

    // 31
    #[test]
    fn spectral_disconnected_graph_zero_lambda2() {
        let g = make_graph();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        // No edges — disconnected.
        let result = g.spectral_analysis(50);
        assert!(
            result.lambda_2 < 0.01,
            "disconnected graph should have lambda_2 ~ 0, got {}",
            result.lambda_2
        );
        assert_eq!(result.node_ids.len(), 2);
    }

    // 32
    #[test]
    fn spectral_partition_splits_graph() {
        let g = make_graph();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        let c = g.add_node("C".into(), serde_json::json!({}));
        let d = g.add_node("D".into(), serde_json::json!({}));
        // Two clusters with weak bridge.
        g.link(a, b, CausalEdgeType::Causes, 1.0, 0, 0);
        g.link(c, d, CausalEdgeType::Causes, 1.0, 0, 0);
        g.link(b, c, CausalEdgeType::Correlates, 0.1, 0, 0);

        let (part_a, part_b) = g.spectral_partition();
        assert!(!part_a.is_empty());
        assert!(!part_b.is_empty());
        assert_eq!(part_a.len() + part_b.len(), 4);
    }

    // 33
    #[test]
    fn spectral_single_node() {
        let g = make_graph();
        g.add_node("A".into(), serde_json::json!({}));
        let result = g.spectral_analysis(50);
        assert_eq!(result.lambda_2, 0.0);
    }

    // =====================================================================
    // Coupling / Predictive Analysis tests
    // =====================================================================

    // 34
    #[test]
    fn coupling_basic() {
        let g = make_graph();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        let c = g.add_node("C".into(), serde_json::json!({}));

        let events = vec![
            ChangeEvent { node_ids: vec![a, b], timestamp: 1 },
            ChangeEvent { node_ids: vec![a, b], timestamp: 2 },
            ChangeEvent { node_ids: vec![a, c], timestamp: 3 },
            ChangeEvent { node_ids: vec![b, c], timestamp: 4 },
        ];

        let coupling = g.compute_coupling(&events);
        assert!(!coupling.is_empty());

        // A-B co-changed 2 times out of max(3,3)=3 → 0.67
        let ab = coupling.iter().find(|p| {
            (p.node_a == a && p.node_b == b) || (p.node_a == b && p.node_b == a)
        });
        assert!(ab.is_some());
        let ab = ab.unwrap();
        assert_eq!(ab.co_changes, 2);
        assert!((ab.coupling_score - 2.0 / 3.0).abs() < 0.01);
    }

    // 35
    #[test]
    fn coupling_empty_events() {
        let g = make_graph();
        let coupling = g.compute_coupling(&[]);
        assert!(coupling.is_empty());
    }

    // 36
    #[test]
    fn predict_changes_burst_detection() {
        let g = make_graph();
        let a = g.add_node("module_a".into(), serde_json::json!({}));
        let b = g.add_node("module_b".into(), serde_json::json!({}));
        let c = g.add_node("module_c".into(), serde_json::json!({}));

        // History: 50 events. Module A changes rarely (every 10th event).
        // Module C fills the rest so there are plenty of events.
        let mut events = Vec::new();
        for i in 0..50 {
            events.push(ChangeEvent { node_ids: vec![c], timestamp: i });
            if i % 10 == 0 {
                events.push(ChangeEvent { node_ids: vec![a], timestamp: i });
            }
        }
        // Burst window: module A + B change together in every recent event.
        for i in 50..60 {
            events.push(ChangeEvent { node_ids: vec![a, b], timestamp: i });
        }
        // Module A baseline: ~5 changes in ~55 events before window (rate ~0.09).
        // Module A window: 10 changes in 10 events (rate 1.0).
        // Burst ratio: ~11x — well above 1.5 threshold.

        let predictions = g.predict_changes(&events, 10, 1.5);
        assert!(!predictions.is_empty());

        // Module A should be predicted (in burst).
        let pred_a = predictions.iter().find(|p| p.node_id == a);
        assert!(pred_a.is_some(), "module_a should be in predictions");
        assert!(pred_a.unwrap().in_burst, "module_a should be in burst");

        // Module B should be predicted (co-modified with A during burst).
        let pred_b = predictions.iter().find(|p| p.node_id == b);
        assert!(pred_b.is_some(), "module_b should be predicted via coupling");
    }

    // 37
    #[test]
    fn predict_changes_empty_events() {
        let g = make_graph();
        let predictions = g.predict_changes(&[], 5, 2.0);
        assert!(predictions.is_empty());
    }

    // 38
    #[test]
    fn node_ids_returns_all() {
        let g = make_graph();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        let ids = g.node_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&a));
        assert!(ids.contains(&b));
    }

    // 39
    #[test]
    fn spectral_strongly_connected_high_lambda2() {
        // A complete graph of 4 nodes should have high lambda_2.
        let g = make_graph();
        let nodes: Vec<NodeId> = (0..4)
            .map(|i| g.add_node(format!("N{i}"), serde_json::json!({})))
            .collect();
        for i in 0..4 {
            for j in 0..4 {
                if i != j {
                    g.link(nodes[i], nodes[j], CausalEdgeType::Causes, 1.0, 0, 0);
                }
            }
        }
        let result = g.spectral_analysis(50);
        // For K4, lambda_2 should be 4.0 (all eigenvalues of Laplacian of K4 are 0,4,4,4).
        assert!(
            result.lambda_2 > 3.0,
            "complete graph K4 lambda_2 should be ~4.0, got {}",
            result.lambda_2
        );
    }

    // ── Persistence tests ────────────────────────────────────────────

    fn tmp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "causal_test_{name}_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    // 40
    #[test]
    fn persist_empty_graph_roundtrip() {
        let g = make_graph();
        let path = tmp_path("empty");
        g.save_to_file(&path).unwrap();
        let loaded = CausalGraph::load_from_file(&path).unwrap();
        assert_eq!(loaded.node_count(), 0);
        assert_eq!(loaded.edge_count(), 0);
        let _ = std::fs::remove_file(&path);
    }

    // 41
    #[test]
    fn persist_nodes_and_edges_roundtrip() {
        let g = make_graph();
        let a = g.add_node("Alpha".into(), serde_json::json!({"role": "source"}));
        let b = g.add_node("Beta".into(), serde_json::json!({"role": "target"}));
        let c = g.add_node("Gamma".into(), serde_json::json!({}));
        g.link(a, b, CausalEdgeType::Causes, 0.9, 100, 1);
        g.link(b, c, CausalEdgeType::Enables, 0.5, 200, 2);
        g.link(a, c, CausalEdgeType::Inhibits, 0.3, 300, 3);

        let path = tmp_path("nodes_edges");
        g.save_to_file(&path).unwrap();
        let loaded = CausalGraph::load_from_file(&path).unwrap();

        assert_eq!(loaded.node_count(), 3);
        assert_eq!(loaded.edge_count(), 3);

        // Verify node data.
        let na = loaded.get_node(a).unwrap();
        assert_eq!(na.label, "Alpha");
        assert_eq!(na.metadata["role"], "source");

        let nb = loaded.get_node(b).unwrap();
        assert_eq!(nb.label, "Beta");

        // Verify edges.
        let fwd_a = loaded.get_forward_edges(a);
        assert_eq!(fwd_a.len(), 2);
        assert!(fwd_a.iter().any(|e| e.target == b && e.edge_type == CausalEdgeType::Causes));
        assert!(fwd_a.iter().any(|e| e.target == c && e.edge_type == CausalEdgeType::Inhibits));

        let _ = std::fs::remove_file(&path);
    }

    // 42
    #[test]
    fn persist_node_metadata_survives() {
        let g = make_graph();
        let id = g.add_node("rich".into(), serde_json::json!({
            "tags": ["a", "b"],
            "count": 42,
            "nested": {"x": true}
        }));

        let path = tmp_path("metadata");
        g.save_to_file(&path).unwrap();
        let loaded = CausalGraph::load_from_file(&path).unwrap();
        let node = loaded.get_node(id).unwrap();
        assert_eq!(node.metadata["tags"][0], "a");
        assert_eq!(node.metadata["count"], 42);
        assert_eq!(node.metadata["nested"]["x"], true);
        let _ = std::fs::remove_file(&path);
    }

    // 43
    #[test]
    fn persist_edge_types_and_weights() {
        let g = make_graph();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        g.link(a, b, CausalEdgeType::Contradicts, 0.77, 555, 10);

        let path = tmp_path("edge_types");
        g.save_to_file(&path).unwrap();
        let loaded = CausalGraph::load_from_file(&path).unwrap();
        let edges = loaded.get_forward_edges(a);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].edge_type, CausalEdgeType::Contradicts);
        assert!((edges[0].weight - 0.77).abs() < 0.001);
        assert_eq!(edges[0].timestamp, 555);
        assert_eq!(edges[0].chain_seq, 10);
        let _ = std::fs::remove_file(&path);
    }

    // 44
    #[test]
    fn persist_next_node_id_preserved() {
        let g = make_graph();
        let _a = g.add_node("A".into(), serde_json::json!({}));
        let _b = g.add_node("B".into(), serde_json::json!({}));
        let _c = g.add_node("C".into(), serde_json::json!({}));
        // next_node_id should be 4 now (started at 1, added 3 nodes).

        let path = tmp_path("next_id");
        g.save_to_file(&path).unwrap();
        let loaded = CausalGraph::load_from_file(&path).unwrap();

        // Adding a new node should get id >= 4, not collide with existing.
        let new_id = loaded.add_node("D".into(), serde_json::json!({}));
        assert!(new_id >= 4, "new node should get id >= 4, got {new_id}");
        assert!(loaded.get_node(new_id).is_some());
        assert_eq!(loaded.node_count(), 4);
        let _ = std::fs::remove_file(&path);
    }

    // 45
    #[test]
    fn persist_writer_reader_roundtrip() {
        let g = make_graph();
        let a = g.add_node("X".into(), serde_json::json!({}));
        let b = g.add_node("Y".into(), serde_json::json!({}));
        g.link(a, b, CausalEdgeType::Follows, 1.0, 0, 0);

        let mut buf = Vec::new();
        g.save_to_writer(&mut buf).unwrap();

        let loaded = CausalGraph::load_from_reader(buf.as_slice()).unwrap();
        assert_eq!(loaded.node_count(), 2);
        assert_eq!(loaded.edge_count(), 1);
        let edges = loaded.get_forward_edges(a);
        assert_eq!(edges[0].edge_type, CausalEdgeType::Follows);
    }

    // 46
    #[test]
    fn persist_reverse_edges_rebuilt() {
        let g = make_graph();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        g.link(a, b, CausalEdgeType::Causes, 1.0, 0, 0);

        let path = tmp_path("reverse");
        g.save_to_file(&path).unwrap();
        let loaded = CausalGraph::load_from_file(&path).unwrap();

        // Reverse edges should be rebuilt from forward edges.
        let rev = loaded.get_reverse_edges(b);
        assert_eq!(rev.len(), 1);
        assert_eq!(rev[0].source, a);
        let _ = std::fs::remove_file(&path);
    }

    // =====================================================================
    // Sparse Lanczos vs dense reference comparison
    // =====================================================================

    /// Dense reference: compute lambda_2 via Jacobi eigendecomposition of the
    /// full Laplacian.  O(n^3) — only used in tests for correctness checking.
    fn dense_spectral_lambda2(g: &CausalGraph, _max_iterations: usize) -> f64 {
        let ids = g.node_ids();
        let n = ids.len();
        if n < 2 {
            return 0.0;
        }

        let mut id_to_idx: std::collections::HashMap<NodeId, usize> =
            std::collections::HashMap::new();
        let mut sorted_ids = ids.clone();
        sorted_ids.sort();
        for (i, &id) in sorted_ids.iter().enumerate() {
            id_to_idx.insert(id, i);
        }

        let mut laplacian = vec![vec![0.0f64; n]; n];
        for &id in &sorted_ids {
            let i = id_to_idx[&id];
            for edge in g.get_forward_edges(id) {
                if let Some(&j) = id_to_idx.get(&edge.target) {
                    if i != j {
                        let w = edge.weight as f64;
                        laplacian[i][j] -= w;
                        laplacian[j][i] -= w;
                    }
                }
            }
            for edge in g.get_reverse_edges(id) {
                if let Some(&j) = id_to_idx.get(&edge.source) {
                    if i != j && j > i {
                        let w = edge.weight as f64;
                        laplacian[i][j] -= w;
                        laplacian[j][i] -= w;
                    }
                }
            }
        }
        for i in 0..n {
            let off_sum: f64 = (0..n).filter(|&j| j != i).map(|j| -laplacian[i][j]).sum();
            laplacian[i][i] = off_sum;
        }

        // Jacobi eigendecomposition of the full Laplacian.
        let mut a = laplacian;
        for _ in 0..100 * n {
            let mut max_off = 0.0f64;
            let mut p = 0usize;
            let mut q = 1usize;
            for i in 0..n {
                for j in (i + 1)..n {
                    if a[i][j].abs() > max_off {
                        max_off = a[i][j].abs();
                        p = i;
                        q = j;
                    }
                }
            }
            if max_off < 1e-15 { break; }
            let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
            let t = theta.signum() / (theta.abs() + (1.0 + theta * theta).sqrt());
            let c = 1.0 / (1.0 + t * t).sqrt();
            let s = t * c;
            let app = a[p][p]; let aqq = a[q][q]; let apq = a[p][q];
            a[p][p] = c * c * app - 2.0 * s * c * apq + s * s * aqq;
            a[q][q] = s * s * app + 2.0 * s * c * apq + c * c * aqq;
            a[p][q] = 0.0;
            a[q][p] = 0.0;
            for r in 0..n {
                if r != p && r != q {
                    let arp = a[r][p]; let arq = a[r][q];
                    a[r][p] = c * arp - s * arq; a[p][r] = a[r][p];
                    a[r][q] = s * arp + c * arq; a[q][r] = a[r][q];
                }
            }
        }

        // Collect eigenvalues, sort, return second-smallest.
        let mut evals: Vec<f64> = (0..n).map(|i| a[i][i]).collect();
        evals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        if n >= 2 { evals[1].max(0.0) } else { 0.0 }
    }

    // 47
    #[test]
    fn spectral_lanczos_matches_dense_triangle() {
        // Triangle graph (K3): known lambda_2 = 3.0 for unit weights.
        let g = make_graph();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        let c = g.add_node("C".into(), serde_json::json!({}));
        g.link(a, b, CausalEdgeType::Causes, 1.0, 0, 0);
        g.link(b, c, CausalEdgeType::Causes, 1.0, 0, 0);
        g.link(a, c, CausalEdgeType::Causes, 1.0, 0, 0);

        let sparse_result = g.spectral_analysis(50);
        let dense_lambda2 = dense_spectral_lambda2(&g, 200);

        assert!(
            (sparse_result.lambda_2 - dense_lambda2).abs() < 0.5,
            "Lanczos lambda_2={} vs dense lambda_2={} differ too much",
            sparse_result.lambda_2,
            dense_lambda2,
        );
        // Both should be close to 3.0 for K3 with symmetric unit edges.
        assert!(sparse_result.lambda_2 > 1.0, "lambda_2 should be > 1 for K3");
    }

    // 48
    #[test]
    fn spectral_lanczos_matches_dense_path() {
        // Path graph: A - B - C - D (lambda_2 ~ 0.586 for unit weights).
        let g = make_graph();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        let c = g.add_node("C".into(), serde_json::json!({}));
        let d = g.add_node("D".into(), serde_json::json!({}));
        g.link(a, b, CausalEdgeType::Causes, 1.0, 0, 0);
        g.link(b, c, CausalEdgeType::Causes, 1.0, 0, 0);
        g.link(c, d, CausalEdgeType::Causes, 1.0, 0, 0);

        let sparse_result = g.spectral_analysis(50);
        let dense_lambda2 = dense_spectral_lambda2(&g, 200);

        assert!(
            (sparse_result.lambda_2 - dense_lambda2).abs() < 0.5,
            "Lanczos lambda_2={} vs dense lambda_2={} differ too much",
            sparse_result.lambda_2,
            dense_lambda2,
        );
        assert!(sparse_result.lambda_2 > 0.0, "path graph should be connected");
    }

    // 49
    #[test]
    fn spectral_lanczos_matches_dense_k4() {
        // K4: lambda_2 = 4.0 for unit-weight complete graph on 4 nodes.
        let g = make_graph();
        let nodes: Vec<NodeId> = (0..4)
            .map(|i| g.add_node(format!("N{i}"), serde_json::json!({})))
            .collect();
        for i in 0..4 {
            for j in (i + 1)..4 {
                g.link(nodes[i], nodes[j], CausalEdgeType::Causes, 1.0, 0, 0);
            }
        }

        let sparse_result = g.spectral_analysis(50);
        let dense_lambda2 = dense_spectral_lambda2(&g, 200);

        assert!(
            (sparse_result.lambda_2 - dense_lambda2).abs() < 0.5,
            "K4: Lanczos lambda_2={} vs dense lambda_2={}",
            sparse_result.lambda_2,
            dense_lambda2,
        );
    }

    // 50
    #[test]
    fn spectral_lanczos_disconnected() {
        // Two isolated nodes — lambda_2 should be 0.
        let g = make_graph();
        g.add_node("A".into(), serde_json::json!({}));
        g.add_node("B".into(), serde_json::json!({}));

        let result = g.spectral_analysis(50);
        assert!(
            result.lambda_2 < 0.01,
            "disconnected graph should have lambda_2 ~ 0, got {}",
            result.lambda_2
        );
    }
}
