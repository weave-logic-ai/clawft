/**
 * SelfAttentionController - Self-Attention Mechanism for Memory Systems
 *
 * Implements self-attention over stored memory entries, allowing the system
 * to compute attention scores that determine which memories are most relevant
 * to a given query vector.
 *
 * Features:
 * - Scaled dot-product attention
 * - Softmax normalization
 * - Top-k filtering with minimum score threshold
 * - Efficient batch processing for large memory sets
 */

import type { VectorBackend } from '../../backends/VectorBackend.js';

/**
 * Configuration for self-attention computation
 */
export interface SelfAttentionConfig {
  /** Number of top results to return */
  topK?: number;
  /** Minimum attention score threshold (0-1) */
  minScore?: number;
  /** Temperature for softmax scaling */
  temperature?: number;
  /** Whether to return attention weights */
  returnWeights?: boolean;
}

/**
 * Individual attention score for a memory entry
 */
export interface AttentionScore {
  /** Memory entry ID */
  id: string;
  /** Computed attention score (0-1 after softmax) */
  score: number;
  /** Raw similarity score before softmax */
  rawScore?: number;
}

/**
 * Result of self-attention computation
 */
export interface SelfAttentionResult {
  /** Attention scores for each memory entry */
  scores: AttentionScore[];
  /** Attended output vector (weighted sum of values) */
  attended: number[];
  /** Execution time in milliseconds */
  executionTimeMs: number;
}

/**
 * Memory entry with embedding for attention computation
 */
export interface MemoryEntry {
  id: string;
  embedding: number[];
  content?: string;
  metadata?: Record<string, any>;
}

/**
 * Self-attention controller for computing attention over memory entries
 */
export class SelfAttentionController {
  private vectorBackend: VectorBackend | null;
  private memoryStore: Map<string, MemoryEntry>;
  private dimension: number;
  private config: SelfAttentionConfig;

  constructor(
    vectorBackend: VectorBackend | null = null,
    config: SelfAttentionConfig = {}
  ) {
    this.vectorBackend = vectorBackend;
    this.memoryStore = new Map();
    this.dimension = 0;
    this.config = {
      topK: 10,
      minScore: 0.0,
      temperature: 1.0,
      returnWeights: false,
      ...config
    };
  }

  /**
   * Add a memory entry to the attention context
   */
  addMemory(entry: MemoryEntry): void {
    if (!entry.id || !entry.embedding || entry.embedding.length === 0) {
      throw new Error('Memory entry must have id and non-empty embedding');
    }

    if (this.dimension === 0) {
      this.dimension = entry.embedding.length;
    } else if (entry.embedding.length !== this.dimension) {
      throw new Error(
        `Embedding dimension mismatch: expected ${this.dimension}, got ${entry.embedding.length}`
      );
    }

    this.memoryStore.set(entry.id, entry);

    // Also add to vector backend if available
    if (this.vectorBackend) {
      const embedding = new Float32Array(entry.embedding);
      this.vectorBackend.insert(entry.id, embedding, entry.metadata);
    }
  }

  /**
   * Remove a memory entry from the attention context
   */
  removeMemory(id: string): boolean {
    return this.memoryStore.delete(id);
  }

  /**
   * Clear all memory entries
   */
  clearMemories(): void {
    this.memoryStore.clear();
    this.dimension = 0;
  }

  /**
   * Compute self-attention for a query vector
   *
   * @param query - Query vector for attention computation
   * @param options - Override default configuration options
   * @returns Self-attention result with scores and attended output
   */
  async computeAttention(
    query: number[],
    options: SelfAttentionConfig = {}
  ): Promise<SelfAttentionResult> {
    const startTime = performance.now();

    // Validate input
    if (!query || !Array.isArray(query)) {
      throw new Error('Query must be a non-null array');
    }

    const config = { ...this.config, ...options };
    const { topK = 10, minScore = 0.0, temperature = 1.0 } = config;

    // Handle empty memory case
    if (this.memoryStore.size === 0) {
      return {
        scores: [],
        attended: [...query],
        executionTimeMs: performance.now() - startTime
      };
    }

    // Validate query dimension
    if (this.dimension > 0 && query.length !== this.dimension) {
      throw new Error(
        `Query dimension mismatch: expected ${this.dimension}, got ${query.length}`
      );
    }

    // Compute raw attention scores using scaled dot-product
    const rawScores: { id: string; score: number; embedding: number[] }[] = [];
    const scale = 1.0 / Math.sqrt(query.length);

    for (const [id, entry] of this.memoryStore.entries()) {
      const dotProduct = this.computeDotProduct(query, entry.embedding);
      const scaledScore = dotProduct * scale / temperature;
      rawScores.push({ id, score: scaledScore, embedding: entry.embedding });
    }

    // Apply softmax normalization
    const normalizedScores = this.applySoftmax(rawScores);

    // Filter by minimum score and take top-k
    const filteredScores = normalizedScores
      .filter(item => item.score >= minScore)
      .sort((a, b) => b.score - a.score)
      .slice(0, topK);

    // Compute attended output (weighted sum of values)
    const attended = this.computeAttendedOutput(query, normalizedScores);

    const executionTimeMs = performance.now() - startTime;

    return {
      scores: filteredScores.map(item => ({
        id: item.id,
        score: item.score,
        rawScore: item.rawScore
      })),
      attended,
      executionTimeMs
    };
  }

  /**
   * Compute dot product between two vectors
   */
  private computeDotProduct(a: number[], b: number[]): number {
    let sum = 0;
    const len = Math.min(a.length, b.length);
    for (let i = 0; i < len; i++) {
      sum += a[i] * b[i];
    }
    return sum;
  }

  /**
   * Apply softmax normalization to scores
   */
  private applySoftmax(
    items: { id: string; score: number; embedding: number[] }[]
  ): { id: string; score: number; rawScore: number; embedding: number[] }[] {
    if (items.length === 0) return [];

    // Find max for numerical stability
    const maxScore = Math.max(...items.map(item => item.score));

    // Compute exp(score - max) for each item
    const expScores = items.map(item => ({
      ...item,
      rawScore: item.score,
      expScore: Math.exp(item.score - maxScore)
    }));

    // Compute sum for normalization
    const sumExp = expScores.reduce((sum, item) => sum + item.expScore, 0);

    // Handle edge case where sum is 0 or very small
    if (sumExp === 0 || !isFinite(sumExp)) {
      return items.map(item => ({
        ...item,
        rawScore: item.score,
        score: 1 / items.length
      }));
    }

    // Normalize to get probabilities
    return expScores.map(item => ({
      id: item.id,
      score: item.expScore / sumExp,
      rawScore: item.rawScore,
      embedding: item.embedding
    }));
  }

  /**
   * Compute attended output as weighted sum of memory embeddings
   */
  private computeAttendedOutput(
    query: number[],
    normalizedScores: { id: string; score: number; embedding: number[] }[]
  ): number[] {
    if (normalizedScores.length === 0) {
      return [...query];
    }

    const dimension = query.length;
    const attended = new Array(dimension).fill(0);

    for (const item of normalizedScores) {
      const weight = item.score;
      for (let i = 0; i < dimension; i++) {
        attended[i] += weight * item.embedding[i];
      }
    }

    return attended;
  }

  /**
   * Get the number of memories in the attention context
   */
  get memoryCount(): number {
    return this.memoryStore.size;
  }

  /**
   * Get the embedding dimension
   */
  get embeddingDimension(): number {
    return this.dimension;
  }

  /**
   * Get statistics about the attention controller
   */
  getStats(): {
    memoryCount: number;
    dimension: number;
    hasVectorBackend: boolean;
  } {
    return {
      memoryCount: this.memoryStore.size,
      dimension: this.dimension,
      hasVectorBackend: this.vectorBackend !== null
    };
  }
}
