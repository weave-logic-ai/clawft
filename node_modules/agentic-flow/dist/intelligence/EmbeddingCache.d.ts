/**
 * EmbeddingCache - Persistent cache for embeddings
 *
 * Makes ONNX embeddings practical by caching across sessions:
 * - First embed: ~400ms (ONNX inference)
 * - Cached embed: ~0.1ms (SQLite lookup) or ~0.01ms (in-memory fallback)
 *
 * Storage: ~/.agentic-flow/embedding-cache.db (if SQLite available)
 *
 * Windows Compatibility:
 * - Falls back to in-memory cache if better-sqlite3 compilation fails
 * - No native module compilation required for basic functionality
 */
export interface CacheStats {
    totalEntries: number;
    hits: number;
    misses: number;
    hitRate: number;
    dbSizeBytes: number;
    oldestEntry: number;
    newestEntry: number;
    backend: 'sqlite' | 'memory' | 'file';
}
export interface CacheConfig {
    maxEntries?: number;
    maxAgeDays?: number;
    dbPath?: string;
    dimension?: number;
    forceMemory?: boolean;
}
/**
 * EmbeddingCache - Auto-selects best available backend
 *
 * Backend priority:
 * 1. Native SQLite (better-sqlite3) - Fastest, 9000x speedup
 * 2. WASM SQLite (sql.js) - Cross-platform with persistence
 * 3. Memory cache - Fallback, no persistence
 */
export declare class EmbeddingCache {
    private backend;
    private config;
    private wasmInitPromise;
    constructor(config?: CacheConfig);
    /**
     * Ensure WASM backend is initialized (if using)
     */
    private ensureInit;
    /**
     * Generate hash key for text + model combination
     */
    private hashKey;
    /**
     * Get embedding from cache
     */
    get(text: string, model?: string): Float32Array | null;
    /**
     * Store embedding in cache
     */
    set(text: string, embedding: Float32Array, model?: string): void;
    /**
     * Check if text is cached
     */
    has(text: string, model?: string): boolean;
    /**
     * Get multiple embeddings at once
     */
    getMany(texts: string[], model?: string): Map<string, Float32Array>;
    /**
     * Store multiple embeddings at once
     */
    setMany(entries: Array<{
        text: string;
        embedding: Float32Array;
    }>, model?: string): void;
    /**
     * Get cache statistics
     */
    getStats(): CacheStats;
    /**
     * Clear all cached embeddings
     */
    clear(): void;
    /**
     * Vacuum database (SQLite only)
     */
    vacuum(): void;
    /**
     * Close database connection
     */
    close(): void;
    /**
     * Check if using SQLite backend (native or WASM)
     */
    isSqliteBackend(): boolean;
    /**
     * Get backend type
     */
    getBackendType(): 'native' | 'wasm' | 'memory';
}
/**
 * Get the singleton embedding cache
 */
export declare function getEmbeddingCache(config?: CacheConfig): EmbeddingCache;
/**
 * Reset the cache singleton (for testing)
 */
export declare function resetEmbeddingCache(): void;
/**
 * Check if SQLite is available
 */
export declare function isSqliteAvailable(): boolean;
//# sourceMappingURL=EmbeddingCache.d.ts.map