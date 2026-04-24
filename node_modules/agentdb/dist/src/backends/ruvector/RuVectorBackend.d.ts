/**
 * RuVectorBackend - High-Performance Vector Storage
 *
 * Implements VectorBackend using @ruvector/core with optional GNN support.
 * Provides <100µs search latency with native SIMD optimizations.
 *
 * Features:
 * - Automatic fallback when @ruvector packages not installed
 * - Separate metadata storage for rich queries
 * - Distance-to-similarity conversion for all metrics
 * - Batch operations for optimal throughput
 * - Persistent storage with separate metadata files
 * - Parallel batch insert with configurable concurrency
 * - Buffer pooling to reduce memory allocations
 * - Adaptive index parameters based on dataset size
 * - Memory-mapped support for large indices
 * - Statistics tracking for performance monitoring
 */
import type { VectorBackend, VectorConfig, SearchResult, SearchOptions, VectorStats } from '../VectorBackend.js';
/** @inline Maximum supported vector dimension for bounds checking */
export declare const MAX_VECTOR_DIMENSION = 4096;
/** @inline Maximum batch size for optimal cache utilization */
export declare const MAX_BATCH_SIZE = 10000;
/** @inline Default cache size for embedding storage */
export declare const DEFAULT_CACHE_SIZE = 10000;
/**
 * Semaphore for controlling concurrent operations
 */
declare class Semaphore {
    private permits;
    private waiting;
    constructor(permits: number);
    acquire(): Promise<void>;
    release(): void;
    get available(): number;
}
/**
 * Buffer pool for reusing Float32Array buffers
 */
declare class BufferPool {
    private pools;
    private maxPoolSize;
    constructor(maxPoolSize?: number);
    /**
     * Acquire a buffer of the specified size
     */
    acquire(size: number): Float32Array;
    /**
     * Release a buffer back to the pool
     */
    release(buffer: Float32Array): void;
    /**
     * Clear all pooled buffers
     */
    clear(): void;
    /**
     * Get pool statistics
     */
    getStats(): {
        totalBuffers: number;
        totalMemory: number;
    };
}
/**
 * Statistics tracker for performance monitoring
 */
interface PerformanceStats {
    insertCount: number;
    insertTotalLatencyMs: number;
    insertMinLatencyMs: number;
    insertMaxLatencyMs: number;
    searchCount: number;
    searchTotalLatencyMs: number;
    searchMinLatencyMs: number;
    searchMaxLatencyMs: number;
    cacheHits: number;
    cacheMisses: number;
    lastMemoryUsage: number;
    indexRebuildCount: number;
}
/**
 * Adaptive index parameters based on dataset size
 */
interface AdaptiveParams {
    M: number;
    efConstruction: number;
    efSearch: number;
}
/**
 * Extended configuration with new options
 */
export interface RuVectorConfig extends VectorConfig {
    /** Concurrency level for parallel batch insert (default: 4) */
    parallelConcurrency?: number;
    /** Buffer pool max size (default: 100) */
    bufferPoolSize?: number;
    /** Enable adaptive index parameters (default: true) */
    adaptiveParams?: boolean;
    /** Enable memory-mapped storage for large indices (default: false) */
    enableMmap?: boolean;
    /** Path for memory-mapped storage */
    mmapPath?: string;
    /** Enable statistics tracking (default: true) */
    enableStats?: boolean;
}
export declare class RuVectorBackend implements VectorBackend {
    readonly name: "ruvector";
    private db;
    private config;
    private metadata;
    private initialized;
    private semaphore;
    private bufferPool;
    private stats;
    private mmapEnabled;
    private mmapBuffer;
    constructor(config: VectorConfig | RuVectorConfig);
    /**
     * Get adaptive HNSW parameters based on expected dataset size
     */
    private getAdaptiveParams;
    /**
     * Initialize RuVector database with optional dependency handling
     */
    initialize(): Promise<void>;
    /**
     * Initialize memory-mapped storage for large indices
     */
    private initializeMmap;
    /**
     * Insert single vector with optional metadata
     */
    insert(id: string, embedding: Float32Array, metadata?: Record<string, any>): void;
    /**
     * Batch insert for optimal performance (sequential)
     */
    insertBatch(items: Array<{
        id: string;
        embedding: Float32Array;
        metadata?: Record<string, any>;
    }>): void;
    /**
     * Parallel batch insert with semaphore-controlled concurrency
     *
     * Processes items in parallel batches for improved throughput on multi-core systems.
     * Uses a semaphore to control the maximum number of concurrent insertions.
     *
     * @param items - Array of items to insert
     * @param options - Configuration options
     * @param options.batchSize - Number of items per batch (default: 100)
     * @param options.concurrency - Max concurrent batches (default: config.parallelConcurrency or 4)
     * @returns Promise that resolves when all items are inserted
     */
    insertBatchParallel(items: Array<{
        id: string;
        embedding: Float32Array;
        metadata?: Record<string, any>;
    }>, options?: {
        batchSize?: number;
        concurrency?: number;
    }): Promise<void>;
    /**
     * Insert using buffer pooling to reduce allocations
     *
     * Acquires a buffer from the pool, copies the embedding data,
     * performs the insert, and returns the buffer to the pool.
     *
     * @param id - Vector ID
     * @param embedding - Vector data (can be regular array or Float32Array)
     * @param metadata - Optional metadata
     */
    insertWithPooledBuffer(id: string, embedding: number[] | Float32Array, metadata?: Record<string, any>): void;
    /**
     * Search for k-nearest neighbors with optional filtering and early termination
     * @inline V8 optimization hint - hot path function
     */
    search(query: Float32Array, k: number, options?: SearchOptions): SearchResult[];
    /**
     * Remove vector by ID
     */
    remove(id: string): boolean;
    /**
     * Get database statistics
     */
    getStats(): VectorStats;
    /**
     * Get extended performance statistics
     *
     * Returns detailed metrics including latencies, cache stats, and buffer pool info.
     */
    getExtendedStats(): {
        basic: VectorStats;
        performance: PerformanceStats;
        bufferPool: {
            totalBuffers: number;
            totalMemory: number;
        };
        config: {
            parallelConcurrency: number;
            adaptiveParams: boolean;
            mmapEnabled: boolean;
        };
    };
    /**
     * Reset performance statistics
     */
    resetStats(): void;
    /**
     * Update index parameters adaptively based on current dataset size
     *
     * This triggers an index rebuild with optimal parameters for the current size.
     * Should be called after significant data changes.
     */
    updateAdaptiveParams(): Promise<void>;
    /**
     * Save index and metadata to disk
     */
    save(savePath: string): Promise<void>;
    /**
     * Load index and metadata from disk
     */
    load(loadPath: string): Promise<void>;
    /**
     * Close and cleanup resources
     */
    close(): void;
    /**
     * Convert distance to similarity score based on metric
     *
     * Cosine: distance is already in [0, 2], where 0 = identical
     * L2: exponential decay for unbounded distances
     * IP: negative inner product, so negate for similarity
     */
    private distanceToSimilarity;
    /**
     * Ensure database is initialized before operations
     */
    private ensureInitialized;
    /**
     * Get the buffer pool instance for advanced use cases
     */
    getBufferPool(): BufferPool;
    /**
     * Get the current concurrency semaphore status
     */
    getConcurrencyStatus(): {
        available: number;
        configured: number;
    };
    /**
     * Check if memory-mapped storage is active
     */
    isMmapEnabled(): boolean;
    /**
     * Get recommended adaptive parameters for a given dataset size
     */
    static getRecommendedParams(datasetSize: number): AdaptiveParams;
}
export { Semaphore, BufferPool };
export type { PerformanceStats, AdaptiveParams };
//# sourceMappingURL=RuVectorBackend.d.ts.map