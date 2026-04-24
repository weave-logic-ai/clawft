/**
 * HNSWIndex - Hierarchical Navigable Small World Index
 *
 * High-performance approximate nearest neighbor (ANN) search using HNSW algorithm.
 * Provides 10-100x speedup over brute-force search for large vector datasets.
 *
 * Features:
 * - HNSW indexing for sub-millisecond search
 * - Automatic index building and management
 * - Configurable M and efConstruction parameters
 * - Persistent index storage
 * - Graceful fallback to brute-force
 * - Multi-distance metric support (cosine, euclidean, ip)
 *
 * Note: hnswlib-node is lazy-loaded to avoid import failures on systems
 * without C++ build tools. Use forceWasm: true in AgentDB config to skip
 * hnswlib entirely and use pure WASM backends.
 */
/**
 * Check if hnswlib-node is available without throwing.
 * Useful for conditional logic before attempting to use HNSWIndex.
 */
export declare function isHnswlibAvailable(): Promise<boolean>;
type Database = any;
export interface HNSWConfig {
    /** Maximum number of connections per layer (default: 16) */
    M: number;
    /** Size of dynamic candidate list during construction (default: 200) */
    efConstruction: number;
    /** Size of dynamic candidate list during search (default: 100) */
    efSearch: number;
    /** Distance metric: 'cosine', 'euclidean', 'ip' (inner product) */
    metric: 'cosine' | 'l2' | 'ip';
    /** Vector dimension */
    dimension: number;
    /** Maximum number of elements in index */
    maxElements: number;
    /** Enable persistent index storage */
    persistIndex: boolean;
    /** Path to store index file */
    indexPath?: string;
    /** Rebuild index threshold (rebuild when updates exceed this percentage) */
    rebuildThreshold: number;
}
export interface HNSWSearchResult {
    id: number;
    distance: number;
    similarity: number;
    metadata?: any;
}
export interface HNSWStats {
    enabled: boolean;
    indexBuilt: boolean;
    numElements: number;
    dimension: number;
    metric: string;
    M: number;
    efConstruction: number;
    efSearch: number;
    lastBuildTime: number | null;
    lastSearchTime: number | null;
    totalSearches: number;
    avgSearchTimeMs: number;
}
export declare class HNSWIndex {
    private db;
    private config;
    private index;
    private vectorCache;
    private idToLabel;
    private labelToId;
    private nextLabel;
    private indexBuilt;
    private updatesSinceLastBuild;
    private totalSearches;
    private totalSearchTime;
    private lastBuildTime;
    private lastSearchTime;
    private pendingPersistentLoad;
    private initializePromise;
    constructor(db: Database, config?: Partial<HNSWConfig>);
    /**
     * Initialize the index asynchronously.
     * Call this after construction if you need to load a persisted index.
     * This is automatically called by buildIndex() and search() if needed.
     */
    initialize(): Promise<void>;
    private doInitialize;
    /**
     * Build HNSW index from database vectors
     */
    buildIndex(tableName?: string): Promise<void>;
    /**
     * Search HNSW index for k-nearest neighbors
     */
    search(query: Float32Array, k: number, options?: {
        threshold?: number;
        filters?: Record<string, any>;
    }): Promise<HNSWSearchResult[]>;
    /**
     * Add a single vector to the index
     */
    addVector(id: number, embedding: Float32Array): void;
    /**
     * Remove a vector from the index
     */
    removeVector(id: number): void;
    /**
     * Check if index needs rebuilding
     */
    needsRebuild(): boolean;
    /**
     * Save index to disk
     */
    private saveIndex;
    /**
     * Load index from disk (async version for lazy loading)
     */
    private loadIndexAsync;
    /**
     * Convert distance to similarity based on metric
     */
    private distanceToSimilarity;
    /**
     * Apply post-filtering to search results
     */
    private applyFilters;
    /**
     * Get index statistics
     */
    getStats(): HNSWStats;
    /**
     * Update efSearch parameter for search quality/speed tradeoff
     */
    setEfSearch(ef: number): void;
    /**
     * Clear index and free memory
     */
    clear(): void;
    /**
     * Check if index is built and ready
     */
    isReady(): boolean;
}
export {};
//# sourceMappingURL=HNSWIndex.d.ts.map