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
export declare class SelfAttentionController {
    private vectorBackend;
    private memoryStore;
    private dimension;
    private config;
    constructor(vectorBackend?: VectorBackend | null, config?: SelfAttentionConfig);
    /**
     * Add a memory entry to the attention context
     */
    addMemory(entry: MemoryEntry): void;
    /**
     * Remove a memory entry from the attention context
     */
    removeMemory(id: string): boolean;
    /**
     * Clear all memory entries
     */
    clearMemories(): void;
    /**
     * Compute self-attention for a query vector
     *
     * @param query - Query vector for attention computation
     * @param options - Override default configuration options
     * @returns Self-attention result with scores and attended output
     */
    computeAttention(query: number[], options?: SelfAttentionConfig): Promise<SelfAttentionResult>;
    /**
     * Compute dot product between two vectors
     */
    private computeDotProduct;
    /**
     * Apply softmax normalization to scores
     */
    private applySoftmax;
    /**
     * Compute attended output as weighted sum of memory embeddings
     */
    private computeAttendedOutput;
    /**
     * Get the number of memories in the attention context
     */
    get memoryCount(): number;
    /**
     * Get the embedding dimension
     */
    get embeddingDimension(): number;
    /**
     * Get statistics about the attention controller
     */
    getStats(): {
        memoryCount: number;
        dimension: number;
        hasVectorBackend: boolean;
    };
}
//# sourceMappingURL=SelfAttentionController.d.ts.map