/**
 * Advanced Features for AgentDB Browser
 *
 * Includes:
 * - GNN (Graph Neural Networks) - Graph attention and message passing
 * - MMR (Maximal Marginal Relevance) - Diversity ranking
 * - SVD (Singular Value Decomposition) - Tensor compression
 * - Batch operations and utilities
 */

// ============================================================================
// GNN (Graph Neural Networks)
// ============================================================================

export interface GNNNode {
  id: number;
  features: Float32Array;
  neighbors: number[];
}

export interface GNNEdge {
  from: number;
  to: number;
  weight: number;
}

export interface GNNConfig {
  hiddenDim: number;
  numHeads: number;        // For multi-head attention
  dropout: number;
  learningRate: number;
  attentionType: 'gat' | 'gcn' | 'sage';
}

/**
 * Graph Neural Network with attention mechanism
 */
export class GraphNeuralNetwork {
  private config: GNNConfig;
  private nodes: Map<number, GNNNode> = new Map();
  private edges: GNNEdge[] = [];
  private attentionWeights: Map<string, number> = new Map();

  constructor(config: Partial<GNNConfig> = {}) {
    this.config = {
      hiddenDim: config.hiddenDim || 64,
      numHeads: config.numHeads || 4,
      dropout: config.dropout || 0.1,
      learningRate: config.learningRate || 0.01,
      attentionType: config.attentionType || 'gat'
    };
  }

  /**
   * Add node to graph
   */
  addNode(id: number, features: Float32Array): void {
    this.nodes.set(id, {
      id,
      features,
      neighbors: []
    });
  }

  /**
   * Add edge to graph
   */
  addEdge(from: number, to: number, weight: number = 1.0): void {
    this.edges.push({ from, to, weight });

    // Update neighbor lists
    const fromNode = this.nodes.get(from);
    const toNode = this.nodes.get(to);

    if (fromNode && !fromNode.neighbors.includes(to)) {
      fromNode.neighbors.push(to);
    }
    if (toNode && !toNode.neighbors.includes(from)) {
      toNode.neighbors.push(from);
    }
  }

  /**
   * Graph Attention Network (GAT) message passing
   */
  graphAttention(nodeId: number): Float32Array {
    const node = this.nodes.get(nodeId);
    if (!node) throw new Error(`Node ${nodeId} not found`);

    const neighbors = node.neighbors;
    if (neighbors.length === 0) {
      return node.features;
    }

    // Multi-head attention
    const headDim = Math.floor(this.config.hiddenDim / this.config.numHeads);
    const aggregated = new Float32Array(this.config.hiddenDim);

    for (let h = 0; h < this.config.numHeads; h++) {
      let attentionSum = 0;
      const headOutput = new Float32Array(headDim);

      // Compute attention scores for each neighbor
      for (const neighborId of neighbors) {
        const neighbor = this.nodes.get(neighborId)!;

        // Attention score: similarity between node and neighbor
        const score = this.computeAttentionScore(
          node.features,
          neighbor.features,
          h
        );

        attentionSum += score;

        // Aggregate neighbor features weighted by attention
        for (let i = 0; i < headDim && i < neighbor.features.length; i++) {
          headOutput[i] += score * neighbor.features[i];
        }
      }

      // Normalize by attention sum
      if (attentionSum > 0) {
        for (let i = 0; i < headDim; i++) {
          headOutput[i] /= attentionSum;
        }
      }

      // Concatenate head outputs
      const offset = h * headDim;
      for (let i = 0; i < headDim; i++) {
        aggregated[offset + i] = headOutput[i];
      }
    }

    // Apply non-linearity (LeakyReLU)
    for (let i = 0; i < aggregated.length; i++) {
      aggregated[i] = aggregated[i] > 0 ? aggregated[i] : 0.01 * aggregated[i];
    }

    return aggregated;
  }

  /**
   * Compute attention score between two nodes
   */
  private computeAttentionScore(
    features1: Float32Array,
    features2: Float32Array,
    head: number
  ): number {
    // Simple dot-product attention
    let score = 0;
    const len = Math.min(features1.length, features2.length);

    for (let i = 0; i < len; i++) {
      score += features1[i] * features2[i];
    }

    // Apply softmax-like normalization
    return Math.exp(score / Math.sqrt(len));
  }

  /**
   * Message passing for all nodes
   */
  messagePass(): Map<number, Float32Array> {
    const newFeatures = new Map<number, Float32Array>();

    for (const [nodeId] of this.nodes) {
      newFeatures.set(nodeId, this.graphAttention(nodeId));
    }

    return newFeatures;
  }

  /**
   * Update node features after message passing
   */
  update(newFeatures: Map<number, Float32Array>): void {
    for (const [nodeId, features] of newFeatures) {
      const node = this.nodes.get(nodeId);
      if (node) {
        node.features = features;
      }
    }
  }

  /**
   * Compute graph embeddings for query enhancement
   */
  computeGraphEmbedding(nodeId: number, hops: number = 2): Float32Array {
    const features = new Map<number, Float32Array>();
    features.set(nodeId, this.nodes.get(nodeId)!.features);

    // Multi-hop message passing
    for (let h = 0; h < hops; h++) {
      const newFeatures = this.messagePass();
      this.update(newFeatures);
    }

    return this.nodes.get(nodeId)!.features;
  }

  /**
   * Get statistics
   */
  getStats() {
    return {
      numNodes: this.nodes.size,
      numEdges: this.edges.length,
      avgDegree: this.edges.length / Math.max(this.nodes.size, 1),
      config: this.config
    };
  }
}

// ============================================================================
// MMR (Maximal Marginal Relevance)
// ============================================================================

export interface MMRConfig {
  lambda: number;  // Trade-off between relevance and diversity (0-1)
  metric: 'cosine' | 'euclidean';
}

/**
 * Maximal Marginal Relevance for diversity ranking
 */
export class MaximalMarginalRelevance {
  private config: MMRConfig;

  constructor(config: Partial<MMRConfig> = {}) {
    this.config = {
      lambda: config.lambda || 0.7,
      metric: config.metric || 'cosine'
    };
  }

  /**
   * Rerank results for diversity
   * @param query Query vector
   * @param candidates Candidate vectors with scores
   * @param k Number of results to return
   * @returns Reranked indices
   */
  rerank(
    query: Float32Array,
    candidates: Array<{ id: number; vector: Float32Array; score: number }>,
    k: number
  ): number[] {
    if (candidates.length === 0) return [];

    const selected: number[] = [];
    const remaining = new Set(candidates.map((_, i) => i));

    // Select first result (highest relevance)
    let bestIdx = 0;
    let bestScore = -Infinity;

    for (let i = 0; i < candidates.length; i++) {
      if (candidates[i].score > bestScore) {
        bestScore = candidates[i].score;
        bestIdx = i;
      }
    }

    selected.push(candidates[bestIdx].id);
    remaining.delete(bestIdx);

    // Iteratively select remaining results
    while (selected.length < k && remaining.size > 0) {
      let bestMMR = -Infinity;
      let bestCandidate = -1;

      for (const idx of remaining) {
        const candidate = candidates[idx];

        // Relevance to query
        const relevance = this.similarity(query, candidate.vector);

        // Maximum similarity to already selected
        let maxSimilarity = -Infinity;
        for (const selectedId of selected) {
          const selectedCandidate = candidates.find(c => c.id === selectedId)!;
          const sim = this.similarity(candidate.vector, selectedCandidate.vector);
          maxSimilarity = Math.max(maxSimilarity, sim);
        }

        // MMR score
        const mmr =
          this.config.lambda * relevance -
          (1 - this.config.lambda) * maxSimilarity;

        if (mmr > bestMMR) {
          bestMMR = mmr;
          bestCandidate = idx;
        }
      }

      if (bestCandidate !== -1) {
        selected.push(candidates[bestCandidate].id);
        remaining.delete(bestCandidate);
      } else {
        break;
      }
    }

    return selected;
  }

  /**
   * Similarity computation
   */
  private similarity(a: Float32Array, b: Float32Array): number {
    if (this.config.metric === 'cosine') {
      return this.cosineSimilarity(a, b);
    } else {
      // Euclidean distance converted to similarity
      const dist = this.euclideanDistance(a, b);
      return 1 / (1 + dist);
    }
  }

  private cosineSimilarity(a: Float32Array, b: Float32Array): number {
    let dotProduct = 0;
    let normA = 0;
    let normB = 0;

    for (let i = 0; i < a.length; i++) {
      dotProduct += a[i] * b[i];
      normA += a[i] * a[i];
      normB += b[i] * b[i];
    }

    return dotProduct / (Math.sqrt(normA) * Math.sqrt(normB));
  }

  private euclideanDistance(a: Float32Array, b: Float32Array): number {
    let sum = 0;
    for (let i = 0; i < a.length; i++) {
      const diff = a[i] - b[i];
      sum += diff * diff;
    }
    return Math.sqrt(sum);
  }

  /**
   * Set lambda (relevance vs diversity trade-off)
   */
  setLambda(lambda: number): void {
    this.config.lambda = Math.max(0, Math.min(1, lambda));
  }
}

// ============================================================================
// SVD (Singular Value Decomposition) for Tensor Compression
// ============================================================================

/**
 * Simple SVD implementation for dimension reduction
 */
export class TensorCompression {
  /**
   * Reduce dimensionality using truncated SVD
   * @param vectors Array of vectors to compress
   * @param targetDim Target dimension
   * @returns Compressed vectors
   */
  static compress(
    vectors: Float32Array[],
    targetDim: number
  ): Float32Array[] {
    if (vectors.length === 0) return [];

    const originalDim = vectors[0].length;
    if (targetDim >= originalDim) return vectors;

    // Create matrix (vectors as rows)
    const matrix = vectors.map(v => Array.from(v));

    // Center the data (subtract mean)
    const mean = this.computeMean(matrix);
    const centered = matrix.map(row =>
      row.map((val, i) => val - mean[i])
    );

    // Compute covariance matrix
    const cov = this.computeCovariance(centered);

    // Compute top k eigenvectors using power iteration
    const eigenvectors = this.powerIteration(cov, targetDim);

    // Project vectors onto eigenvectors
    const compressed = centered.map(row => {
      const projected = new Float32Array(targetDim);
      for (let i = 0; i < targetDim; i++) {
        let sum = 0;
        for (let j = 0; j < originalDim; j++) {
          sum += row[j] * eigenvectors[i][j];
        }
        projected[i] = sum;
      }
      return projected;
    });

    return compressed;
  }

  /**
   * Compute mean vector
   */
  private static computeMean(matrix: number[][]): number[] {
    const n = matrix.length;
    const dim = matrix[0].length;
    const mean = new Array(dim).fill(0);

    for (const row of matrix) {
      for (let i = 0; i < dim; i++) {
        mean[i] += row[i];
      }
    }

    return mean.map(v => v / n);
  }

  /**
   * Compute covariance matrix
   */
  private static computeCovariance(matrix: number[][]): number[][] {
    const n = matrix.length;
    const dim = matrix[0].length;
    const cov: number[][] = Array.from({ length: dim }, () =>
      new Array(dim).fill(0)
    );

    for (let i = 0; i < dim; i++) {
      for (let j = 0; j <= i; j++) {
        let sum = 0;
        for (const row of matrix) {
          sum += row[i] * row[j];
        }
        cov[i][j] = cov[j][i] = sum / n;
      }
    }

    return cov;
  }

  /**
   * Power iteration for computing top eigenvectors
   */
  private static powerIteration(
    matrix: number[][],
    k: number,
    iterations: number = 100
  ): number[][] {
    const dim = matrix.length;
    const eigenvectors: number[][] = [];

    for (let i = 0; i < k; i++) {
      // Random initialization
      let v = new Array(dim).fill(0).map(() => Math.random() - 0.5);

      // Power iteration
      for (let iter = 0; iter < iterations; iter++) {
        // Multiply by matrix
        const newV = new Array(dim).fill(0);
        for (let r = 0; r < dim; r++) {
          for (let c = 0; c < dim; c++) {
            newV[r] += matrix[r][c] * v[c];
          }
        }

        // Orthogonalize against previous eigenvectors
        for (const prev of eigenvectors) {
          let dot = 0;
          for (let j = 0; j < dim; j++) {
            dot += newV[j] * prev[j];
          }
          for (let j = 0; j < dim; j++) {
            newV[j] -= dot * prev[j];
          }
        }

        // Normalize
        let norm = 0;
        for (const val of newV) {
          norm += val * val;
        }
        norm = Math.sqrt(norm);

        if (norm < 1e-10) break;

        v = newV.map(val => val / norm);
      }

      eigenvectors.push(v);
    }

    return eigenvectors;
  }
}

// ============================================================================
// Batch Operations
// ============================================================================

/**
 * Efficient batch processing utilities
 */
export class BatchProcessor {
  /**
   * Batch cosine similarity computation
   */
  static batchCosineSimilarity(
    query: Float32Array,
    vectors: Float32Array[]
  ): Float32Array {
    const similarities = new Float32Array(vectors.length);

    // Precompute query norm
    let queryNorm = 0;
    for (let i = 0; i < query.length; i++) {
      queryNorm += query[i] * query[i];
    }
    queryNorm = Math.sqrt(queryNorm);

    // Compute similarities
    for (let v = 0; v < vectors.length; v++) {
      const vector = vectors[v];
      let dotProduct = 0;
      let vectorNorm = 0;

      for (let i = 0; i < query.length; i++) {
        dotProduct += query[i] * vector[i];
        vectorNorm += vector[i] * vector[i];
      }

      vectorNorm = Math.sqrt(vectorNorm);
      similarities[v] = dotProduct / (queryNorm * vectorNorm);
    }

    return similarities;
  }

  /**
   * Batch vector normalization
   */
  static batchNormalize(vectors: Float32Array[]): Float32Array[] {
    return vectors.map(v => {
      let norm = 0;
      for (let i = 0; i < v.length; i++) {
        norm += v[i] * v[i];
      }
      norm = Math.sqrt(norm);

      const normalized = new Float32Array(v.length);
      for (let i = 0; i < v.length; i++) {
        normalized[i] = v[i] / norm;
      }
      return normalized;
    });
  }
}
