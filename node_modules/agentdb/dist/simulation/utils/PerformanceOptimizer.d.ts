/**
 * Performance Optimizer for AgentDB Simulations
 *
 * Optimizations:
 * - Batch database operations
 * - Intelligent caching
 * - Parallel agent execution
 * - Memory pooling
 * - Query optimization
 * - Connection pooling
 */
export declare class PerformanceOptimizer {
    private batchQueue;
    private batchSize;
    private cache;
    private metrics;
    constructor(options?: {
        batchSize?: number;
    });
    /**
     * Add operation to batch queue
     */
    queueOperation(operation: () => Promise<any>): void;
    /**
     * Execute all queued operations in parallel batches
     */
    executeBatch(): Promise<any[]>;
    /**
     * Cache data with TTL
     */
    setCache(key: string, data: any, ttl?: number): void;
    /**
     * Get cached data
     */
    getCache(key: string): any | null;
    /**
     * Clear expired cache entries
     */
    clearExpiredCache(): void;
    /**
     * Get cache statistics
     */
    getCacheStats(): {
        size: number;
        hits: number;
        misses: number;
        hitRate: string;
    };
    /**
     * Get performance metrics
     */
    getMetrics(): {
        batchOperations: number;
        totalOperations: number;
        avgLatency: string;
        cacheStats: {
            size: number;
            hits: number;
            misses: number;
            hitRate: string;
        };
    };
    /**
     * Reset metrics
     */
    resetMetrics(): void;
    /**
     * Clear all cache
     */
    clearCache(): void;
}
/**
 * Parallel execution utility
 */
export declare function executeParallel<T>(tasks: Array<() => Promise<T>>, concurrency?: number): Promise<T[]>;
/**
 * Memory pooling for agent objects
 */
export declare class AgentPool<T> {
    private pool;
    private factory;
    private maxSize;
    constructor(factory: () => T, maxSize?: number);
    /**
     * Get agent from pool or create new
     */
    acquire(): T;
    /**
     * Return agent to pool
     */
    release(agent: T): void;
    /**
     * Clear pool
     */
    clear(): void;
    /**
     * Get pool size
     */
    size(): number;
}
/**
 * Query optimizer for database operations
 */
export declare class QueryOptimizer {
    private queryCache;
    /**
     * Optimize Cypher query with caching
     */
    executeOptimized(queryFn: () => Promise<any>, cacheKey?: string, ttl?: number): Promise<any>;
    /**
     * Clear query cache
     */
    clearCache(): void;
}
//# sourceMappingURL=PerformanceOptimizer.d.ts.map