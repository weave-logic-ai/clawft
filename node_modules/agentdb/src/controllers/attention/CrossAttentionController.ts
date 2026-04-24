/**
 * CrossAttentionController - Cross-Attention Mechanism for Memory Systems
 *
 * Implements cross-attention between a query and different memory contexts,
 * allowing the system to attend to multiple namespaces or memory sources
 * and integrate information across them.
 *
 * Features:
 * - Multiple context namespace support
 * - Query-context integration
 * - Flexible aggregation strategies
 * - Efficient retrieval with vector backends
 */

import type { VectorBackend } from '../../backends/VectorBackend.js';

/**
 * Configuration for cross-attention computation
 */
export interface CrossAttentionConfig {
  /** Number of top results to return per context */
  topK?: number;
  /** Minimum attention score threshold (0-1) */
  minScore?: number;
  /** Temperature for softmax scaling */
  temperature?: number;
  /** Aggregation strategy for multi-context attention */
  aggregation?: 'average' | 'max' | 'weighted';
}

/**
 * Individual attention score for a context entry
 */
export interface CrossAttentionScore {
  /** Context entry ID */
  id: string;
  /** Context namespace */
  context: string;
  /** Computed attention score (0-1 after softmax) */
  score: number;
  /** Raw similarity score before softmax */
  rawScore?: number;
}

/**
 * Result of cross-attention computation
 */
export interface CrossAttentionResult {
  /** Attention scores for each context entry */
  scores: CrossAttentionScore[];
  /** Attended output vector (integrated from context) */
  attended: number[];
  /** Per-context contribution weights */
  contextWeights?: Record<string, number>;
  /** Execution time in milliseconds */
  executionTimeMs: number;
}

/**
 * Memory entry with embedding for cross-attention
 */
export interface ContextEntry {
  id: string;
  embedding: number[];
  content?: string;
  metadata?: Record<string, any>;
}

/**
 * Cross-attention controller for computing attention across memory contexts
 */
export class CrossAttentionController {
  private vectorBackend: VectorBackend | null;
  private contextStores: Map<string, Map<string, ContextEntry>>;
  private dimension: number;
  private config: CrossAttentionConfig;

  constructor(
    vectorBackend: VectorBackend | null = null,
    config: CrossAttentionConfig = {}
  ) {
    this.vectorBackend = vectorBackend;
    this.contextStores = new Map();
    this.dimension = 0;
    this.config = {
      topK: 10,
      minScore: 0.0,
      temperature: 1.0,
      aggregation: 'average',
      ...config
    };
  }

  /**
   * Add an entry to a specific context namespace
   */
  addToContext(contextName: string, entry: ContextEntry): void {
    if (!entry.id || !entry.embedding || entry.embedding.length === 0) {
      throw new Error('Context entry must have id and non-empty embedding');
    }

    if (this.dimension === 0) {
      this.dimension = entry.embedding.length;
    } else if (entry.embedding.length !== this.dimension) {
      throw new Error(
        `Embedding dimension mismatch: expected ${this.dimension}, got ${entry.embedding.length}`
      );
    }

    if (!this.contextStores.has(contextName)) {
      this.contextStores.set(contextName, new Map());
    }

    this.contextStores.get(contextName)!.set(entry.id, entry);

    // Add to vector backend with context metadata
    if (this.vectorBackend) {
      const embedding = new Float32Array(entry.embedding);
      this.vectorBackend.insert(entry.id, embedding, {
        ...entry.metadata,
        __context: contextName
      });
    }
  }

  /**
   * Remove an entry from a context
   */
  removeFromContext(contextName: string, id: string): boolean {
    const store = this.contextStores.get(contextName);
    if (!store) return false;
    return store.delete(id);
  }

  /**
   * Clear a specific context
   */
  clearContext(contextName: string): void {
    this.contextStores.delete(contextName);
  }

  /**
   * Clear all contexts
   */
  clearAllContexts(): void {
    this.contextStores.clear();
    this.dimension = 0;
  }

  /**
   * List available context namespaces
   */
  listContexts(): string[] {
    return Array.from(this.contextStores.keys());
  }

  /**
   * Compute cross-attention between query and a specific context
   *
   * @param query - Query vector for attention computation
   * @param contextName - Name of the context to attend to
   * @param options - Override default configuration options
   * @returns Cross-attention result with scores and attended output
   */
  async computeCrossAttention(
    query: number[],
    contextName: string,
    options: CrossAttentionConfig = {}
  ): Promise<CrossAttentionResult> {
    const startTime = performance.now();

    // Validate input
    if (!query || !Array.isArray(query)) {
      throw new Error('Query must be a non-null array');
    }

    const config = { ...this.config, ...options };
    const { topK = 10, minScore = 0.0, temperature = 1.0 } = config;

    // Get context store
    const contextStore = this.contextStores.get(contextName);
    if (!contextStore || contextStore.size === 0) {
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

    // Compute raw attention scores
    const rawScores: { id: string; context: string; score: number; embedding: number[] }[] = [];
    const scale = 1.0 / Math.sqrt(query.length);

    for (const [id, entry] of contextStore.entries()) {
      const dotProduct = this.computeDotProduct(query, entry.embedding);
      const scaledScore = dotProduct * scale / temperature;
      rawScores.push({
        id,
        context: contextName,
        score: scaledScore,
        embedding: entry.embedding
      });
    }

    // Apply softmax normalization
    const normalizedScores = this.applySoftmax(rawScores);

    // Filter by minimum score and take top-k
    const filteredScores = normalizedScores
      .filter(item => item.score >= minScore)
      .sort((a, b) => b.score - a.score)
      .slice(0, topK);

    // Compute attended output
    const attended = this.computeAttendedOutput(query, normalizedScores);

    const executionTimeMs = performance.now() - startTime;

    return {
      scores: filteredScores.map(item => ({
        id: item.id,
        context: item.context,
        score: item.score,
        rawScore: item.rawScore
      })),
      attended,
      executionTimeMs
    };
  }

  /**
   * Compute cross-attention across multiple contexts
   *
   * @param query - Query vector for attention computation
   * @param contextNames - Names of contexts to attend to (all if empty)
   * @param options - Override default configuration options
   * @returns Cross-attention result with aggregated output
   */
  async computeMultiContextAttention(
    query: number[],
    contextNames: string[] = [],
    options: CrossAttentionConfig = {}
  ): Promise<CrossAttentionResult> {
    const startTime = performance.now();

    // Use all contexts if none specified
    const contexts = contextNames.length > 0
      ? contextNames
      : Array.from(this.contextStores.keys());

    if (contexts.length === 0) {
      return {
        scores: [],
        attended: [...query],
        executionTimeMs: performance.now() - startTime
      };
    }

    const config = { ...this.config, ...options };
    const { aggregation = 'average' } = config;

    // Compute attention for each context
    const contextResults: CrossAttentionResult[] = [];
    for (const contextName of contexts) {
      const result = await this.computeCrossAttention(query, contextName, options);
      contextResults.push(result);
    }

    // Aggregate results based on strategy
    const allScores: CrossAttentionScore[] = [];
    const contextWeights: Record<string, number> = {};

    for (let i = 0; i < contextResults.length; i++) {
      const result = contextResults[i];
      const contextName = contexts[i];

      allScores.push(...result.scores);

      // Compute context weight based on aggregation strategy
      const totalScore = result.scores.reduce((sum, s) => sum + s.score, 0);
      contextWeights[contextName] = totalScore;
    }

    // Normalize context weights
    const totalWeight = Object.values(contextWeights).reduce((sum, w) => sum + w, 0);
    if (totalWeight > 0) {
      for (const key of Object.keys(contextWeights)) {
        contextWeights[key] /= totalWeight;
      }
    }

    // Aggregate attended outputs
    const attended = this.aggregateAttendedOutputs(
      query,
      contextResults.map(r => r.attended),
      Object.values(contextWeights),
      aggregation
    );

    const executionTimeMs = performance.now() - startTime;

    return {
      scores: allScores,
      attended,
      contextWeights,
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
    items: { id: string; context: string; score: number; embedding: number[] }[]
  ): { id: string; context: string; score: number; rawScore: number; embedding: number[] }[] {
    if (items.length === 0) return [];

    const maxScore = Math.max(...items.map(item => item.score));
    const expScores = items.map(item => ({
      ...item,
      rawScore: item.score,
      expScore: Math.exp(item.score - maxScore)
    }));

    const sumExp = expScores.reduce((sum, item) => sum + item.expScore, 0);

    if (sumExp === 0 || !isFinite(sumExp)) {
      return items.map(item => ({
        ...item,
        rawScore: item.score,
        score: 1 / items.length
      }));
    }

    return expScores.map(item => ({
      id: item.id,
      context: item.context,
      score: item.expScore / sumExp,
      rawScore: item.rawScore,
      embedding: item.embedding
    }));
  }

  /**
   * Compute attended output as weighted sum
   */
  private computeAttendedOutput(
    query: number[],
    normalizedScores: { score: number; embedding: number[] }[]
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
   * Aggregate attended outputs from multiple contexts
   */
  private aggregateAttendedOutputs(
    query: number[],
    outputs: number[][],
    weights: number[],
    strategy: 'average' | 'max' | 'weighted'
  ): number[] {
    if (outputs.length === 0) {
      return [...query];
    }

    const dimension = query.length;
    const result = new Array(dimension).fill(0);

    switch (strategy) {
      case 'max':
        for (let i = 0; i < dimension; i++) {
          result[i] = Math.max(...outputs.map(o => o[i] || 0));
        }
        break;

      case 'weighted':
        for (let j = 0; j < outputs.length; j++) {
          const weight = weights[j] || (1 / outputs.length);
          for (let i = 0; i < dimension; i++) {
            result[i] += weight * (outputs[j][i] || 0);
          }
        }
        break;

      case 'average':
      default:
        for (let j = 0; j < outputs.length; j++) {
          for (let i = 0; i < dimension; i++) {
            result[i] += (outputs[j][i] || 0) / outputs.length;
          }
        }
        break;
    }

    return result;
  }

  /**
   * Get the total number of entries across all contexts
   */
  get totalEntryCount(): number {
    let count = 0;
    for (const store of this.contextStores.values()) {
      count += store.size;
    }
    return count;
  }

  /**
   * Get statistics about the cross-attention controller
   */
  getStats(): {
    contextCount: number;
    totalEntries: number;
    dimension: number;
    hasVectorBackend: boolean;
    contextsInfo: Record<string, number>;
  } {
    const contextsInfo: Record<string, number> = {};
    for (const [name, store] of this.contextStores.entries()) {
      contextsInfo[name] = store.size;
    }

    return {
      contextCount: this.contextStores.size,
      totalEntries: this.totalEntryCount,
      dimension: this.dimension,
      hasVectorBackend: this.vectorBackend !== null,
      contextsInfo
    };
  }
}
