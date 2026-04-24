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
export class QueryOptimizer {
    db;
    cache;
    stats;
    config;
    constructor(db, config) {
        this.db = db;
        this.cache = new Map();
        this.stats = new Map();
        this.config = {
            maxSize: 1000,
            ttl: 60000, // 1 minute default
            enabled: true,
            ...config
        };
    }
    /**
     * Execute query with caching
     */
    query(sql, params = [], cacheKey) {
        const key = cacheKey || this.generateCacheKey(sql, params);
        const startTime = Date.now();
        // Check cache
        if (this.config.enabled && this.cache.has(key)) {
            const cached = this.cache.get(key);
            if (Date.now() - cached.timestamp < this.config.ttl) {
                this.recordStats(sql, Date.now() - startTime, true);
                return cached.result;
            }
            else {
                this.cache.delete(key);
            }
        }
        // Execute query
        const stmt = this.db.prepare(sql);
        const result = params.length > 0 ? stmt.all(...params) : stmt.all();
        const executionTime = Date.now() - startTime;
        this.recordStats(sql, executionTime, false);
        // Cache result
        if (this.config.enabled) {
            this.cacheResult(key, result);
        }
        return result;
    }
    /**
     * Execute query that returns single row
     */
    queryOne(sql, params = [], cacheKey) {
        const results = this.query(sql, params, cacheKey);
        return results[0];
    }
    /**
     * Execute write operation (no caching)
     */
    execute(sql, params = []) {
        const startTime = Date.now();
        const stmt = this.db.prepare(sql);
        const result = params.length > 0 ? stmt.run(...params) : stmt.run();
        this.recordStats(sql, Date.now() - startTime, false);
        // Invalidate relevant cache entries
        this.invalidateCache(sql);
        return result;
    }
    /**
     * Batch insert optimization
     */
    batchInsert(table, columns, rows) {
        const placeholders = columns.map(() => '?').join(', ');
        const sql = `INSERT INTO ${table} (${columns.join(', ')}) VALUES (${placeholders})`;
        const transaction = this.db.transaction((rows) => {
            const stmt = this.db.prepare(sql);
            for (const row of rows) {
                stmt.run(...row);
            }
        });
        const startTime = Date.now();
        transaction(rows);
        this.recordStats(`BATCH INSERT ${table}`, Date.now() - startTime, false);
    }
    /**
     * Analyze query plan
     */
    analyzeQuery(sql) {
        const plan = this.db.prepare(`EXPLAIN QUERY PLAN ${sql}`).all();
        const planText = plan.map((row) => row.detail).join(' ');
        const usesIndex = planText.toLowerCase().includes('index');
        const hasFullScan = planText.toLowerCase().includes('scan');
        // Simple cost estimation
        let estimatedCost = 1;
        if (hasFullScan)
            estimatedCost *= 10;
        if (!usesIndex)
            estimatedCost *= 5;
        return {
            plan: planText,
            usesIndex,
            estimatedCost
        };
    }
    /**
     * Get optimization suggestions
     */
    getSuggestions() {
        const suggestions = [];
        // Analyze frequently run queries
        const frequentQueries = Array.from(this.stats.values())
            .filter(s => s.executionCount > 100)
            .sort((a, b) => b.totalTime - a.totalTime)
            .slice(0, 10);
        for (const stat of frequentQueries) {
            if (stat.avgTime > 50) {
                const analysis = this.analyzeQuery(stat.query);
                if (!analysis.usesIndex) {
                    suggestions.push(`Slow query (${stat.avgTime.toFixed(1)}ms avg): Consider adding index for:\n${stat.query}`);
                }
                if (stat.cacheHits === 0 && stat.executionCount > 50) {
                    suggestions.push(`Frequently run query without cache hits: ${stat.query.substring(0, 50)}...`);
                }
            }
        }
        // Check cache efficiency
        const totalHits = Array.from(this.stats.values()).reduce((sum, s) => sum + s.cacheHits, 0);
        const totalMisses = Array.from(this.stats.values()).reduce((sum, s) => sum + s.cacheMisses, 0);
        const hitRate = totalHits / (totalHits + totalMisses) || 0;
        if (hitRate < 0.3 && totalHits + totalMisses > 1000) {
            suggestions.push(`Low cache hit rate (${(hitRate * 100).toFixed(1)}%). Consider increasing cache size or TTL.`);
        }
        return suggestions;
    }
    /**
     * Get query statistics
     */
    getStats() {
        return Array.from(this.stats.values())
            .sort((a, b) => b.totalTime - a.totalTime);
    }
    /**
     * Clear cache
     */
    clearCache() {
        this.cache.clear();
    }
    /**
     * Get cache statistics
     */
    getCacheStats() {
        const totalHits = Array.from(this.stats.values()).reduce((sum, s) => sum + s.cacheHits, 0);
        const totalMisses = Array.from(this.stats.values()).reduce((sum, s) => sum + s.cacheMisses, 0);
        return {
            size: this.cache.size,
            hitRate: totalHits / (totalHits + totalMisses) || 0,
            totalHits,
            totalMisses
        };
    }
    // ========================================================================
    // Private Methods
    // ========================================================================
    generateCacheKey(sql, params) {
        return `${sql}:${JSON.stringify(params)}`;
    }
    cacheResult(key, result) {
        if (this.cache.size >= this.config.maxSize) {
            // Simple LRU: remove oldest entry
            const oldestKey = this.cache.keys().next().value;
            if (oldestKey) {
                this.cache.delete(oldestKey);
            }
        }
        this.cache.set(key, {
            result,
            timestamp: Date.now()
        });
    }
    invalidateCache(sql) {
        // Invalidate cache entries related to modified tables
        const tables = this.extractTables(sql);
        for (const [key] of this.cache) {
            for (const table of tables) {
                if (key.toLowerCase().includes(table.toLowerCase())) {
                    this.cache.delete(key);
                }
            }
        }
    }
    extractTables(sql) {
        const matches = sql.match(/(?:FROM|INTO|UPDATE|JOIN)\s+(\w+)/gi);
        if (!matches)
            return [];
        return matches
            .map(m => m.split(/\s+/)[1])
            .filter((v, i, a) => a.indexOf(v) === i); // unique
    }
    recordStats(sql, time, cacheHit) {
        const key = sql.substring(0, 100); // Use first 100 chars as key
        if (!this.stats.has(key)) {
            this.stats.set(key, {
                query: sql,
                executionCount: 0,
                totalTime: 0,
                avgTime: 0,
                cacheHits: 0,
                cacheMisses: 0
            });
        }
        const stat = this.stats.get(key);
        stat.executionCount++;
        stat.totalTime += time;
        stat.avgTime = stat.totalTime / stat.executionCount;
        if (cacheHit) {
            stat.cacheHits++;
        }
        else {
            stat.cacheMisses++;
        }
    }
}
//# sourceMappingURL=QueryOptimizer.js.map