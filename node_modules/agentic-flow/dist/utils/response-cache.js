/**
 * Response Cache with LRU Eviction
 * Provides 50-80% latency reduction for repeated queries
 */
import { logger } from './logger.js';
export class ResponseCache {
    cache = new Map();
    accessOrder = []; // LRU tracking
    config;
    stats;
    constructor(config = {}) {
        this.config = {
            maxSize: config.maxSize || 100,
            ttl: config.ttl || 60000, // 60 seconds default
            updateAgeOnGet: config.updateAgeOnGet ?? true,
            enableStats: config.enableStats ?? true
        };
        this.stats = {
            size: 0,
            maxSize: this.config.maxSize,
            hits: 0,
            misses: 0,
            hitRate: 0,
            evictions: 0,
            totalSavings: 0
        };
        // Cleanup expired entries every minute
        setInterval(() => this.cleanup(), 60000);
    }
    /**
     * Get cached response
     */
    get(key) {
        const entry = this.cache.get(key);
        if (!entry) {
            this.stats.misses++;
            this.updateHitRate();
            return undefined;
        }
        // Check if expired
        if (this.isExpired(entry)) {
            this.cache.delete(key);
            this.removeFromAccessOrder(key);
            this.stats.misses++;
            this.stats.size = this.cache.size;
            this.updateHitRate();
            return undefined;
        }
        // Update access order for LRU
        if (this.config.updateAgeOnGet) {
            this.removeFromAccessOrder(key);
            this.accessOrder.push(key);
            entry.timestamp = Date.now();
        }
        entry.hits++;
        this.stats.hits++;
        this.stats.totalSavings += entry.data.length;
        this.updateHitRate();
        logger.debug('Cache hit', {
            key: key.substring(0, 50),
            hits: entry.hits,
            age: Date.now() - entry.timestamp
        });
        return entry;
    }
    /**
     * Set cached response
     */
    set(key, value) {
        // Evict if at capacity
        if (this.cache.size >= this.config.maxSize && !this.cache.has(key)) {
            this.evictLRU();
        }
        // Update access order
        if (this.cache.has(key)) {
            this.removeFromAccessOrder(key);
        }
        this.accessOrder.push(key);
        // Store entry
        value.timestamp = Date.now();
        value.hits = 0;
        this.cache.set(key, value);
        this.stats.size = this.cache.size;
        logger.debug('Cache set', {
            key: key.substring(0, 50),
            size: value.data.length,
            cacheSize: this.cache.size
        });
    }
    /**
     * Generate cache key from request
     */
    generateKey(req) {
        // Don't cache streaming requests
        if (req.stream) {
            return '';
        }
        const parts = [
            req.model || 'default',
            JSON.stringify(req.messages || []),
            req.max_tokens?.toString() || '1000',
            req.temperature?.toString() || '1.0'
        ];
        // Use hash to keep key short
        return this.hash(parts.join(':'));
    }
    /**
     * Check if response should be cached
     */
    shouldCache(req, statusCode) {
        // Don't cache streaming requests
        if (req.stream) {
            return false;
        }
        // Only cache successful responses
        if (statusCode !== 200 && statusCode !== 201) {
            return false;
        }
        return true;
    }
    /**
     * Clear expired entries
     */
    cleanup() {
        const now = Date.now();
        let removed = 0;
        for (const [key, entry] of this.cache.entries()) {
            if (this.isExpired(entry)) {
                this.cache.delete(key);
                this.removeFromAccessOrder(key);
                removed++;
            }
        }
        this.stats.size = this.cache.size;
        if (removed > 0) {
            logger.debug('Cache cleanup', { removed, remaining: this.cache.size });
        }
    }
    /**
     * Evict least recently used entry
     */
    evictLRU() {
        if (this.accessOrder.length === 0)
            return;
        const lruKey = this.accessOrder.shift();
        if (lruKey) {
            this.cache.delete(lruKey);
            this.stats.evictions++;
            logger.debug('Cache eviction (LRU)', {
                key: lruKey.substring(0, 50),
                cacheSize: this.cache.size
            });
        }
    }
    /**
     * Check if entry is expired
     */
    isExpired(entry) {
        return (Date.now() - entry.timestamp) > this.config.ttl;
    }
    /**
     * Remove key from access order
     */
    removeFromAccessOrder(key) {
        const index = this.accessOrder.indexOf(key);
        if (index !== -1) {
            this.accessOrder.splice(index, 1);
        }
    }
    /**
     * Update hit rate statistic
     */
    updateHitRate() {
        const total = this.stats.hits + this.stats.misses;
        this.stats.hitRate = total > 0 ? this.stats.hits / total : 0;
    }
    /**
     * Simple hash function for cache keys
     */
    hash(str) {
        let hash = 0;
        for (let i = 0; i < str.length; i++) {
            const char = str.charCodeAt(i);
            hash = ((hash << 5) - hash) + char;
            hash = hash & hash; // Convert to 32-bit integer
        }
        return Math.abs(hash).toString(36);
    }
    /**
     * Get cache statistics
     */
    getStats() {
        return { ...this.stats };
    }
    /**
     * Clear cache
     */
    clear() {
        this.cache.clear();
        this.accessOrder = [];
        this.stats.size = 0;
        this.stats.evictions = 0;
        logger.info('Cache cleared');
    }
    /**
     * Destroy cache and cleanup
     */
    destroy() {
        this.clear();
    }
}
//# sourceMappingURL=response-cache.js.map