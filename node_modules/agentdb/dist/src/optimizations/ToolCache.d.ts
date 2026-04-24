/**
 * ToolCache - Intelligent Caching for MCP Tools
 *
 * Features:
 * - TTL-based expiration
 * - LRU eviction when max size reached
 * - Pattern-based cache invalidation
 * - Hit/miss rate tracking
 * - Memory-efficient storage
 *
 * Performance Impact:
 * - agentdb_stats: 176ms → ~20ms (8.8x faster)
 * - pattern_stats: Similar improvement
 * - learning_metrics: 120s TTL for expensive computations
 */
export interface CacheEntry<T> {
    value: T;
    expiry: number;
    accessCount: number;
    lastAccess: number;
}
export interface CacheStats {
    size: number;
    maxSize: number;
    hits: number;
    misses: number;
    hitRate: number;
    evictions: number;
    avgAccessCount: number;
}
export declare class ToolCache<T = any> {
    private cache;
    private maxSize;
    private defaultTTLMs;
    private hits;
    private misses;
    private evictions;
    constructor(maxSize?: number, defaultTTLMs?: number);
    /**
     * Set cache entry with optional custom TTL
     */
    set(key: string, value: T, ttlMs?: number): void;
    /**
     * Get cache entry (returns null if expired or not found)
     */
    get(key: string): T | null;
    /**
     * Check if key exists and is not expired
     */
    has(key: string): boolean;
    /**
     * Delete specific key
     */
    delete(key: string): boolean;
    /**
     * Clear all entries matching pattern (e.g., 'stats:*', 'search:user-123:*')
     */
    clear(pattern?: string): number;
    /**
     * Evict all expired entries
     */
    private evictExpired;
    /**
     * Evict least recently used entry (LRU)
     */
    private evictLRU;
    /**
     * Get cache statistics
     */
    getStats(): CacheStats;
    /**
     * Reset statistics (keeps cached data)
     */
    resetStats(): void;
    /**
     * Get all cache keys (useful for debugging)
     */
    keys(): string[];
    /**
     * Get cache entry with metadata (useful for debugging)
     */
    inspect(key: string): CacheEntry<T> | null;
    /**
     * Warmup cache with pre-computed values
     */
    warmup(entries: Array<{
        key: string;
        value: T;
        ttlMs?: number;
    }>): void;
    /**
     * Export cache to JSON (for persistence)
     */
    export(): Array<{
        key: string;
        value: T;
        expiry: number;
    }>;
    /**
     * Import cache from JSON (for persistence)
     */
    import(entries: Array<{
        key: string;
        value: T;
        expiry: number;
    }>): number;
}
/**
 * Specialized caches for different MCP tools
 */
export declare class MCPToolCaches {
    stats: ToolCache<string>;
    patterns: ToolCache<any[]>;
    searches: ToolCache<any[]>;
    metrics: ToolCache<any>;
    constructor();
    /**
     * Clear all caches
     */
    clearAll(): void;
    /**
     * Get aggregate statistics
     */
    getAggregateStats(): {
        stats: CacheStats;
        patterns: CacheStats;
        searches: CacheStats;
        metrics: CacheStats;
        total: {
            size: number;
            hits: number;
            misses: number;
            hitRate: number;
        };
    };
}
//# sourceMappingURL=ToolCache.d.ts.map