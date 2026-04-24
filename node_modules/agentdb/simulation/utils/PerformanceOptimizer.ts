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

export class PerformanceOptimizer {
  private batchQueue: Array<() => Promise<any>> = [];
  private batchSize: number = 100;
  private cache: Map<string, { data: any; timestamp: number; ttl: number }> = new Map();
  private metrics: {
    batchOperations: number;
    cacheHits: number;
    cacheMisses: number;
    totalLatency: number;
    operations: number;
  } = {
    batchOperations: 0,
    cacheHits: 0,
    cacheMisses: 0,
    totalLatency: 0,
    operations: 0
  };

  constructor(options: { batchSize?: number } = {}) {
    this.batchSize = options.batchSize || 100;
  }

  /**
   * Add operation to batch queue
   */
  queueOperation(operation: () => Promise<any>): void {
    this.batchQueue.push(operation);
  }

  /**
   * Execute all queued operations in parallel batches
   */
  async executeBatch(): Promise<any[]> {
    if (this.batchQueue.length === 0) return [];

    const startTime = performance.now();
    const results: any[] = [];

    // Process in batches to avoid overwhelming the system
    for (let i = 0; i < this.batchQueue.length; i += this.batchSize) {
      const batch = this.batchQueue.slice(i, i + this.batchSize);
      const batchResults = await Promise.all(batch.map(op => op()));
      results.push(...batchResults);
      this.metrics.batchOperations++;
    }

    this.batchQueue = [];
    const endTime = performance.now();
    this.metrics.totalLatency += endTime - startTime;
    this.metrics.operations += results.length;

    return results;
  }

  /**
   * Cache data with TTL
   */
  setCache(key: string, data: any, ttl: number = 60000): void {
    this.cache.set(key, {
      data,
      timestamp: Date.now(),
      ttl
    });
  }

  /**
   * Get cached data
   */
  getCache(key: string): any | null {
    const cached = this.cache.get(key);

    if (!cached) {
      this.metrics.cacheMisses++;
      return null;
    }

    // Check if expired
    if (Date.now() - cached.timestamp > cached.ttl) {
      this.cache.delete(key);
      this.metrics.cacheMisses++;
      return null;
    }

    this.metrics.cacheHits++;
    return cached.data;
  }

  /**
   * Clear expired cache entries
   */
  clearExpiredCache(): void {
    const now = Date.now();
    for (const [key, value] of this.cache.entries()) {
      if (now - value.timestamp > value.ttl) {
        this.cache.delete(key);
      }
    }
  }

  /**
   * Get cache statistics
   */
  getCacheStats() {
    const hitRate = this.metrics.cacheHits + this.metrics.cacheMisses > 0
      ? (this.metrics.cacheHits / (this.metrics.cacheHits + this.metrics.cacheMisses)) * 100
      : 0;

    return {
      size: this.cache.size,
      hits: this.metrics.cacheHits,
      misses: this.metrics.cacheMisses,
      hitRate: hitRate.toFixed(2) + '%'
    };
  }

  /**
   * Get performance metrics
   */
  getMetrics() {
    const avgLatency = this.metrics.operations > 0
      ? this.metrics.totalLatency / this.metrics.operations
      : 0;

    return {
      batchOperations: this.metrics.batchOperations,
      totalOperations: this.metrics.operations,
      avgLatency: avgLatency.toFixed(2) + 'ms',
      cacheStats: this.getCacheStats()
    };
  }

  /**
   * Reset metrics
   */
  resetMetrics(): void {
    this.metrics = {
      batchOperations: 0,
      cacheHits: 0,
      cacheMisses: 0,
      totalLatency: 0,
      operations: 0
    };
  }

  /**
   * Clear all cache
   */
  clearCache(): void {
    this.cache.clear();
  }
}

/**
 * Parallel execution utility
 */
export async function executeParallel<T>(
  tasks: Array<() => Promise<T>>,
  concurrency: number = 10
): Promise<T[]> {
  const results: T[] = [];

  for (let i = 0; i < tasks.length; i += concurrency) {
    const batch = tasks.slice(i, i + concurrency);
    const batchResults = await Promise.all(batch.map(task => task()));
    results.push(...batchResults);
  }

  return results;
}

/**
 * Memory pooling for agent objects
 */
export class AgentPool<T> {
  private pool: T[] = [];
  private factory: () => T;
  private maxSize: number;

  constructor(factory: () => T, maxSize: number = 100) {
    this.factory = factory;
    this.maxSize = maxSize;
  }

  /**
   * Get agent from pool or create new
   */
  acquire(): T {
    if (this.pool.length > 0) {
      return this.pool.pop()!;
    }
    return this.factory();
  }

  /**
   * Return agent to pool
   */
  release(agent: T): void {
    if (this.pool.length < this.maxSize) {
      this.pool.push(agent);
    }
  }

  /**
   * Clear pool
   */
  clear(): void {
    this.pool = [];
  }

  /**
   * Get pool size
   */
  size(): number {
    return this.pool.length;
  }
}

/**
 * Query optimizer for database operations
 */
export class QueryOptimizer {
  private queryCache: Map<string, any> = new Map();

  /**
   * Optimize Cypher query with caching
   */
  async executeOptimized(
    queryFn: () => Promise<any>,
    cacheKey?: string,
    ttl: number = 5000
  ): Promise<any> {
    if (cacheKey) {
      const cached = this.queryCache.get(cacheKey);
      if (cached && Date.now() - cached.timestamp < ttl) {
        return cached.data;
      }
    }

    const result = await queryFn();

    if (cacheKey) {
      this.queryCache.set(cacheKey, {
        data: result,
        timestamp: Date.now()
      });
    }

    return result;
  }

  /**
   * Clear query cache
   */
  clearCache(): void {
    this.queryCache.clear();
  }
}
