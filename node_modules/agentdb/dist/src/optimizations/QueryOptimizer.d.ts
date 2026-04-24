/**
 * QueryOptimizer - Advanced Query Optimization for AgentDB
 *
 * Implements:
 * - Query result caching with TTL
 * - Prepared statement pooling
 * - Batch operation optimization
 * - Index usage analysis
 * - Query plan analysis
 */
type Database = any;
export interface CacheConfig {
    maxSize: number;
    ttl: number;
    enabled: boolean;
}
export interface QueryStats {
    query: string;
    executionCount: number;
    totalTime: number;
    avgTime: number;
    cacheHits: number;
    cacheMisses: number;
}
export declare class QueryOptimizer {
    private db;
    private cache;
    private stats;
    private config;
    constructor(db: Database, config?: Partial<CacheConfig>);
    /**
     * Execute query with caching
     */
    query<T = any>(sql: string, params?: any[], cacheKey?: string): T;
    /**
     * Execute query that returns single row
     */
    queryOne<T = any>(sql: string, params?: any[], cacheKey?: string): T | undefined;
    /**
     * Execute write operation (no caching)
     */
    execute(sql: string, params?: any[]): any;
    /**
     * Batch insert optimization
     */
    batchInsert(table: string, columns: string[], rows: any[][]): void;
    /**
     * Analyze query plan
     */
    analyzeQuery(sql: string): {
        plan: string;
        usesIndex: boolean;
        estimatedCost: number;
    };
    /**
     * Get optimization suggestions
     */
    getSuggestions(): string[];
    /**
     * Get query statistics
     */
    getStats(): QueryStats[];
    /**
     * Clear cache
     */
    clearCache(): void;
    /**
     * Get cache statistics
     */
    getCacheStats(): {
        size: number;
        hitRate: number;
        totalHits: number;
        totalMisses: number;
    };
    private generateCacheKey;
    private cacheResult;
    private invalidateCache;
    private extractTables;
    private recordStats;
}
export {};
//# sourceMappingURL=QueryOptimizer.d.ts.map