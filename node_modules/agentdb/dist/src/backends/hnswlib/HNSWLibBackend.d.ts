/**
 * HNSWLibBackend - Vector backend adapter for hnswlib-node
 *
 * Wraps existing HNSWIndex controller to implement VectorBackend interface.
 * Handles string ID to numeric label mapping required by hnswlib.
 *
 * Features:
 * - String ID support (maps to hnswlib numeric labels)
 * - Metadata storage alongside vectors
 * - Persistent save/load with mappings
 * - Backward compatible with existing HNSWIndex usage
 *
 * Note: hnswlib-node doesn't support true deletion - removed IDs are
 * tracked but vectors remain until rebuild.
 */
import type { VectorBackend, VectorConfig, SearchResult, SearchOptions, VectorStats } from '../VectorBackend.js';
export declare class HNSWLibBackend implements VectorBackend {
    readonly name: "hnswlib";
    private index;
    private config;
    private idToLabel;
    private labelToId;
    private metadata;
    private nextLabel;
    private deletedIds;
    constructor(config: VectorConfig);
    /**
     * Initialize the HNSW index
     * Must be called after construction
     */
    initialize(): Promise<void>;
    /**
     * Insert a single vector with optional metadata
     */
    insert(id: string, embedding: Float32Array, metadata?: Record<string, any>): void;
    /**
     * Insert multiple vectors in batch
     */
    insertBatch(items: Array<{
        id: string;
        embedding: Float32Array;
        metadata?: Record<string, any>;
    }>): void;
    /**
     * Search for k-nearest neighbors
     */
    search(query: Float32Array, k: number, options?: SearchOptions): SearchResult[];
    /**
     * Remove a vector by ID
     * Note: hnswlib doesn't support true deletion - we mark as deleted
     */
    remove(id: string): boolean;
    /**
     * Get backend statistics
     */
    getStats(): VectorStats;
    /**
     * Save index to disk with mappings
     */
    save(savePath: string): Promise<void>;
    /**
     * Load index from disk with mappings
     */
    load(loadPath: string): Promise<void>;
    /**
     * Close and cleanup resources
     */
    close(): void;
    /**
     * Convert distance to similarity based on metric
     * Maps to [0, 1] range where 1 = most similar
     */
    private distanceToSimilarity;
    /**
     * Apply metadata filters (post-filtering)
     */
    private applyFilters;
    /**
     * Check if needs rebuilding (for backward compat with HNSWIndex)
     * @param updateThreshold - Percentage of deletes to trigger rebuild (default: 0.1)
     */
    needsRebuild(updateThreshold?: number): boolean;
    /**
     * Update efSearch parameter
     */
    setEfSearch(ef: number): void;
    /**
     * Check if backend is ready
     */
    isReady(): boolean;
}
//# sourceMappingURL=HNSWLibBackend.d.ts.map