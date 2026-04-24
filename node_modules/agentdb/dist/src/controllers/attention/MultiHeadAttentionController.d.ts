/**
 * MultiHeadAttentionController - Multi-Head Attention for Memory Systems
 *
 * Implements multi-head attention that projects queries and memory entries
 * into multiple subspaces, computes attention in each, and aggregates the
 * results. This allows the model to attend to different aspects of the
 * information simultaneously.
 *
 * Features:
 * - Configurable number of attention heads
 * - Multiple aggregation strategies (average, max, concat)
 * - Per-head dimension control
 * - Parallel attention computation
 */
import type { VectorBackend } from '../../backends/VectorBackend.js';
/**
 * Configuration for multi-head attention computation
 */
export interface MultiHeadAttentionConfig {
    /** Number of attention heads */
    numHeads?: number;
    /** Dimension per head (auto-calculated if not specified) */
    headDim?: number;
    /** Number of top results to return per head */
    topK?: number;
    /** Minimum attention score threshold (0-1) */
    minScore?: number;
    /** Temperature for softmax scaling */
    temperature?: number;
    /** Aggregation strategy for combining head outputs */
    aggregation?: 'average' | 'max' | 'concat' | 'weighted';
}
/**
 * Attention output from a single head
 */
export interface HeadAttentionOutput {
    /** Head index */
    headIndex: number;
    /** Attended output from this head */
    attended: number[];
    /** Top attention scores for this head */
    topScores: {
        id: string;
        score: number;
    }[];
}
/**
 * Result of multi-head attention computation
 */
export interface MultiHeadAttentionResult {
    /** Per-head attention outputs */
    heads: HeadAttentionOutput[];
    /** Aggregated attended output */
    attended: number[];
    /** Overall top scores across all heads */
    aggregatedScores?: {
        id: string;
        score: number;
    }[];
    /** Execution time in milliseconds */
    executionTimeMs: number;
}
/**
 * Memory entry with embedding for multi-head attention
 */
export interface MemoryEntry {
    id: string;
    embedding: number[];
    content?: string;
    metadata?: Record<string, any>;
}
/**
 * Multi-head attention controller for computing parallel attention patterns
 */
export declare class MultiHeadAttentionController {
    private vectorBackend;
    private memoryStore;
    private dimension;
    private config;
    private headProjections;
    constructor(vectorBackend?: VectorBackend | null, config?: MultiHeadAttentionConfig);
    /**
     * Initialize head projections for a given dimension
     */
    private initializeProjections;
    /**
     * Add a memory entry to the attention context
     */
    addMemory(entry: MemoryEntry): void;
    /**
     * Remove a memory entry
     */
    removeMemory(id: string): boolean;
    /**
     * Clear all memories
     */
    clearMemories(): void;
    /**
     * Project a vector using a head's projection matrix
     */
    private projectVector;
    /**
     * Compute multi-head attention for a query vector
     *
     * @param query - Query vector for attention computation
     * @param options - Override default configuration options
     * @returns Multi-head attention result with per-head outputs and aggregated result
     */
    computeMultiHeadAttention(query: number[], options?: MultiHeadAttentionConfig): Promise<MultiHeadAttentionResult>;
    /**
     * Compute attention for a single head
     */
    private computeHeadAttention;
    /**
     * Apply softmax to scores
     */
    private applySoftmax;
    /**
     * Aggregate outputs from all attention heads
     */
    private aggregateHeadOutputs;
    /**
     * Reconstruct full-dimension vector from head output
     */
    private reconstructFromHead;
    /**
     * Get statistics about the controller
     */
    getStats(): {
        memoryCount: number;
        dimension: number;
        numHeads: number;
        headDim: number;
        hasVectorBackend: boolean;
    };
}
//# sourceMappingURL=MultiHeadAttentionController.d.ts.map