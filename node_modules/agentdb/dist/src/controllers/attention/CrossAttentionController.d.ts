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
export declare class CrossAttentionController {
    private vectorBackend;
    private contextStores;
    private dimension;
    private config;
    constructor(vectorBackend?: VectorBackend | null, config?: CrossAttentionConfig);
    /**
     * Add an entry to a specific context namespace
     */
    addToContext(contextName: string, entry: ContextEntry): void;
    /**
     * Remove an entry from a context
     */
    removeFromContext(contextName: string, id: string): boolean;
    /**
     * Clear a specific context
     */
    clearContext(contextName: string): void;
    /**
     * Clear all contexts
     */
    clearAllContexts(): void;
    /**
     * List available context namespaces
     */
    listContexts(): string[];
    /**
     * Compute cross-attention between query and a specific context
     *
     * @param query - Query vector for attention computation
     * @param contextName - Name of the context to attend to
     * @param options - Override default configuration options
     * @returns Cross-attention result with scores and attended output
     */
    computeCrossAttention(query: number[], contextName: string, options?: CrossAttentionConfig): Promise<CrossAttentionResult>;
    /**
     * Compute cross-attention across multiple contexts
     *
     * @param query - Query vector for attention computation
     * @param contextNames - Names of contexts to attend to (all if empty)
     * @param options - Override default configuration options
     * @returns Cross-attention result with aggregated output
     */
    computeMultiContextAttention(query: number[], contextNames?: string[], options?: CrossAttentionConfig): Promise<CrossAttentionResult>;
    /**
     * Compute dot product between two vectors
     */
    private computeDotProduct;
    /**
     * Apply softmax normalization to scores
     */
    private applySoftmax;
    /**
     * Compute attended output as weighted sum
     */
    private computeAttendedOutput;
    /**
     * Aggregate attended outputs from multiple contexts
     */
    private aggregateAttendedOutputs;
    /**
     * Get the total number of entries across all contexts
     */
    get totalEntryCount(): number;
    /**
     * Get statistics about the cross-attention controller
     */
    getStats(): {
        contextCount: number;
        totalEntries: number;
        dimension: number;
        hasVectorBackend: boolean;
        contextsInfo: Record<string, number>;
    };
}
//# sourceMappingURL=CrossAttentionController.d.ts.map