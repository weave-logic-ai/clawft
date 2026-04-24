/**
 * Response Cache with LRU Eviction
 * Provides 50-80% latency reduction for repeated queries
 */
export interface CacheConfig {
    maxSize: number;
    ttl: number;
    updateAgeOnGet: boolean;
    enableStats: boolean;
}
export interface CachedResponse {
    data: Buffer;
    headers: Record<string, string>;
    statusCode: number;
    timestamp: number;
    hits: number;
}
export interface CacheStats {
    size: number;
    maxSize: number;
    hits: number;
    misses: number;
    hitRate: number;
    evictions: number;
    totalSavings: number;
}
export declare class ResponseCache {
    private cache;
    private accessOrder;
    private config;
    private stats;
    constructor(config?: Partial<CacheConfig>);
    /**
     * Get cached response
     */
    get(key: string): CachedResponse | undefined;
    /**
     * Set cached response
     */
    set(key: string, value: CachedResponse): void;
    /**
     * Generate cache key from request
     */
    generateKey(req: {
        model?: string;
        messages?: unknown[];
        max_tokens?: number;
        temperature?: number;
        stream?: boolean;
    }): string;
    /**
     * Check if response should be cached
     */
    shouldCache(req: {
        stream?: boolean;
    }, statusCode: number): boolean;
    /**
     * Clear expired entries
     */
    private cleanup;
    /**
     * Evict least recently used entry
     */
    private evictLRU;
    /**
     * Check if entry is expired
     */
    private isExpired;
    /**
     * Remove key from access order
     */
    private removeFromAccessOrder;
    /**
     * Update hit rate statistic
     */
    private updateHitRate;
    /**
     * Simple hash function for cache keys
     */
    private hash;
    /**
     * Get cache statistics
     */
    getStats(): CacheStats;
    /**
     * Clear cache
     */
    clear(): void;
    /**
     * Destroy cache and cleanup
     */
    destroy(): void;
}
//# sourceMappingURL=response-cache.d.ts.map