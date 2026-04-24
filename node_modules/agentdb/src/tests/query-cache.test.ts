/**
 * QueryCache Test Suite
 *
 * Comprehensive tests for LRU query cache functionality
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { QueryCache } from '../core/QueryCache.js';

describe('QueryCache', () => {
  let cache: QueryCache;

  beforeEach(() => {
    cache = new QueryCache({
      maxSize: 10,
      defaultTTL: 1000, // 1 second for testing
      enabled: true,
    });
  });

  describe('Basic Operations', () => {
    it('should store and retrieve values', () => {
      cache.set('key1', 'value1');
      expect(cache.get('key1')).toBe('value1');
    });

    it('should return undefined for non-existent keys', () => {
      expect(cache.get('nonexistent')).toBeUndefined();
    });

    it('should check if key exists', () => {
      cache.set('key1', 'value1');
      expect(cache.has('key1')).toBe(true);
      expect(cache.has('nonexistent')).toBe(false);
    });

    it('should delete entries', () => {
      cache.set('key1', 'value1');
      expect(cache.delete('key1')).toBe(true);
      expect(cache.has('key1')).toBe(false);
      expect(cache.delete('nonexistent')).toBe(false);
    });

    it('should clear all entries', () => {
      cache.set('key1', 'value1');
      cache.set('key2', 'value2');
      cache.clear();
      expect(cache.has('key1')).toBe(false);
      expect(cache.has('key2')).toBe(false);
    });
  });

  describe('Key Generation', () => {
    it('should generate consistent keys for same inputs', () => {
      const key1 = cache.generateKey('SELECT * FROM table', ['param1', 'param2'], 'test');
      const key2 = cache.generateKey('SELECT * FROM table', ['param1', 'param2'], 'test');
      expect(key1).toBe(key2);
    });

    it('should generate different keys for different inputs', () => {
      const key1 = cache.generateKey('SELECT * FROM table1', [], 'test');
      const key2 = cache.generateKey('SELECT * FROM table2', [], 'test');
      expect(key1).not.toBe(key2);
    });

    it('should include category in key', () => {
      const key1 = cache.generateKey('SELECT *', [], 'episodes');
      const key2 = cache.generateKey('SELECT *', [], 'skills');
      expect(key1).not.toBe(key2);
      expect(key1.startsWith('episodes:')).toBe(true);
      expect(key2.startsWith('skills:')).toBe(true);
    });
  });

  describe('LRU Eviction', () => {
    it('should evict least recently used entry when full', () => {
      // Fill cache to capacity
      for (let i = 0; i < 10; i++) {
        cache.set(`key${i}`, `value${i}`);
      }

      // Access key5 to make it recently used
      cache.get('key5');

      // Add new entry, should evict key0 (least recently used)
      cache.set('key10', 'value10');

      expect(cache.has('key0')).toBe(false);
      expect(cache.has('key5')).toBe(true);
      expect(cache.has('key10')).toBe(true);
    });

    it('should update LRU order on access', () => {
      cache.set('key1', 'value1');
      cache.set('key2', 'value2');
      cache.set('key3', 'value3');

      // Access key1 to make it most recently used
      cache.get('key1');

      // Fill remaining capacity
      for (let i = 4; i <= 10; i++) {
        cache.set(`key${i}`, `value${i}`);
      }

      // Add one more, should evict key2 (now least recently used)
      cache.set('key11', 'value11');

      expect(cache.has('key1')).toBe(true); // Still there because we accessed it
      expect(cache.has('key2')).toBe(false); // Evicted
    });
  });

  describe('TTL (Time To Live)', () => {
    it('should expire entries after TTL', async () => {
      cache.set('key1', 'value1', 100); // 100ms TTL

      expect(cache.get('key1')).toBe('value1');

      // Wait for expiration
      await new Promise((resolve) => setTimeout(resolve, 150));

      expect(cache.get('key1')).toBeUndefined();
    });

    it('should use default TTL when not specified', async () => {
      cache.set('key1', 'value1'); // Uses default 1000ms TTL

      expect(cache.get('key1')).toBe('value1');

      // Should still be valid after 500ms
      await new Promise((resolve) => setTimeout(resolve, 500));
      expect(cache.get('key1')).toBe('value1');
    });

    it('should prune expired entries', async () => {
      cache.set('key1', 'value1', 100);
      cache.set('key2', 'value2', 5000);

      await new Promise((resolve) => setTimeout(resolve, 150));

      const pruned = cache.pruneExpired();
      expect(pruned).toBe(1);
      expect(cache.has('key1')).toBe(false);
      expect(cache.has('key2')).toBe(true);
    });
  });

  describe('Statistics', () => {
    it('should track cache hits', () => {
      cache.set('key1', 'value1');
      cache.get('key1'); // hit
      cache.get('key1'); // hit

      const stats = cache.getStatistics();
      expect(stats.hits).toBe(2);
      expect(stats.misses).toBe(0);
      expect(stats.hitRate).toBe(100);
    });

    it('should track cache misses', () => {
      cache.get('nonexistent'); // miss
      cache.get('nonexistent'); // miss

      const stats = cache.getStatistics();
      expect(stats.hits).toBe(0);
      expect(stats.misses).toBe(2);
      expect(stats.hitRate).toBe(0);
    });

    it('should calculate hit rate correctly', () => {
      cache.set('key1', 'value1');
      cache.get('key1'); // hit
      cache.get('nonexistent'); // miss
      cache.get('key1'); // hit

      const stats = cache.getStatistics();
      expect(stats.hits).toBe(2);
      expect(stats.misses).toBe(1);
      expect(stats.hitRate).toBeCloseTo(66.67, 1);
    });

    it('should track evictions', () => {
      for (let i = 0; i < 11; i++) {
        cache.set(`key${i}`, `value${i}`);
      }

      const stats = cache.getStatistics();
      expect(stats.evictions).toBe(1);
    });

    it('should track memory usage', () => {
      cache.set('key1', 'short');
      cache.set('key2', 'a much longer string value');

      const stats = cache.getStatistics();
      expect(stats.memoryUsed).toBeGreaterThan(0);
    });

    it('should track entries by category', () => {
      const key1 = cache.generateKey('sql1', [], 'episodes');
      const key2 = cache.generateKey('sql2', [], 'episodes');
      const key3 = cache.generateKey('sql3', [], 'skills');

      cache.set(key1, 'value1');
      cache.set(key2, 'value2');
      cache.set(key3, 'value3');

      const stats = cache.getStatistics();
      expect(stats.entriesByCategory['episodes']).toBe(2);
      expect(stats.entriesByCategory['skills']).toBe(1);
    });

    it('should reset statistics', () => {
      cache.set('key1', 'value1');
      cache.get('key1');
      cache.get('nonexistent');

      cache.resetStatistics();

      const stats = cache.getStatistics();
      expect(stats.hits).toBe(0);
      expect(stats.misses).toBe(0);
    });
  });

  describe('Category Invalidation', () => {
    it('should invalidate entries by category', () => {
      const episodeKey1 = cache.generateKey('sql1', [], 'episodes');
      const episodeKey2 = cache.generateKey('sql2', [], 'episodes');
      const skillKey = cache.generateKey('sql3', [], 'skills');

      cache.set(episodeKey1, 'value1');
      cache.set(episodeKey2, 'value2');
      cache.set(skillKey, 'value3');

      const invalidated = cache.invalidateCategory('episodes');

      expect(invalidated).toBe(2);
      expect(cache.has(episodeKey1)).toBe(false);
      expect(cache.has(episodeKey2)).toBe(false);
      expect(cache.has(skillKey)).toBe(true);
    });

    it('should return 0 when no entries match category', () => {
      const invalidated = cache.invalidateCategory('nonexistent');
      expect(invalidated).toBe(0);
    });
  });

  describe('Complex Data Types', () => {
    it('should cache arrays', () => {
      const data = [1, 2, 3, 4, 5];
      cache.set('array', data);
      expect(cache.get('array')).toEqual(data);
    });

    it('should cache objects', () => {
      const data = { name: 'test', value: 42 };
      cache.set('object', data);
      expect(cache.get('object')).toEqual(data);
    });

    it('should cache Float32Array', () => {
      const data = new Float32Array([1.1, 2.2, 3.3]);
      cache.set('float32', data);
      expect(cache.get('float32')).toEqual(data);
    });

    it('should cache buffers', () => {
      const data = Buffer.from('test data');
      cache.set('buffer', data);
      expect(cache.get('buffer')).toEqual(data);
    });

    it('should not cache results exceeding max size', () => {
      const largeCache = new QueryCache({
        maxSize: 10,
        maxResultSize: 100, // 100 bytes max
      });

      const smallData = 'small';
      const largeData = 'x'.repeat(200); // 400 bytes (UTF-16)

      largeCache.set('small', smallData);
      largeCache.set('large', largeData);

      expect(largeCache.has('small')).toBe(true);
      expect(largeCache.has('large')).toBe(false);
    });
  });

  describe('Configuration', () => {
    it('should respect enabled flag', () => {
      const disabledCache = new QueryCache({ enabled: false });
      disabledCache.set('key1', 'value1');
      expect(disabledCache.get('key1')).toBeUndefined();
    });

    it('should enable/disable caching dynamically', () => {
      cache.set('key1', 'value1');
      expect(cache.get('key1')).toBe('value1');

      cache.setEnabled(false);
      expect(cache.get('key1')).toBeUndefined();

      cache.setEnabled(true);
      cache.set('key2', 'value2');
      expect(cache.get('key2')).toBe('value2');
    });

    it('should return current configuration', () => {
      const config = cache.getConfig();
      expect(config.maxSize).toBe(10);
      expect(config.defaultTTL).toBe(1000);
      expect(config.enabled).toBe(true);
    });

    it('should update configuration', () => {
      cache.updateConfig({ maxSize: 20, defaultTTL: 2000 });
      const config = cache.getConfig();
      expect(config.maxSize).toBe(20);
      expect(config.defaultTTL).toBe(2000);
    });
  });

  describe('Cache Warming', () => {
    it('should execute warmup function', async () => {
      let executed = false;
      await cache.warm(async () => {
        executed = true;
      });
      expect(executed).toBe(true);
    });

    it('should pre-populate cache during warmup', async () => {
      await cache.warm(async (c) => {
        c.set('preloaded1', 'value1');
        c.set('preloaded2', 'value2');
      });

      expect(cache.get('preloaded1')).toBe('value1');
      expect(cache.get('preloaded2')).toBe('value2');
    });
  });

  describe('Thread Safety Simulation', () => {
    it('should handle concurrent operations', async () => {
      const promises = [];

      // Simulate concurrent reads and writes
      for (let i = 0; i < 20; i++) {
        promises.push(
          Promise.resolve().then(() => {
            cache.set(`key${i}`, `value${i}`);
            return cache.get(`key${i}`);
          })
        );
      }

      const results = await Promise.all(promises);
      // Check that we got valid results (may not all be present due to LRU)
      expect(results.filter((r) => r !== undefined).length).toBeGreaterThan(0);
    });
  });

  describe('Memory Efficiency', () => {
    it('should estimate size correctly for strings', () => {
      cache.set('key1', 'test'); // ~8 bytes
      const stats = cache.getStatistics();
      expect(stats.memoryUsed).toBeGreaterThan(0);
      expect(stats.memoryUsed).toBeLessThan(100);
    });

    it('should estimate size correctly for numbers', () => {
      cache.set('key1', 42);
      const stats = cache.getStatistics();
      expect(stats.memoryUsed).toBe(8); // 8 bytes for number
    });

    it('should estimate size correctly for arrays', () => {
      cache.set('key1', [1, 2, 3, 4, 5]);
      const stats = cache.getStatistics();
      expect(stats.memoryUsed).toBe(40); // 5 numbers * 8 bytes
    });
  });

  describe('Edge Cases', () => {
    it('should handle null values', () => {
      cache.set('key1', null);
      expect(cache.get('key1')).toBeNull();
    });

    it('should handle undefined values', () => {
      cache.set('key1', undefined);
      expect(cache.get('key1')).toBeUndefined();
    });

    it('should handle empty strings', () => {
      cache.set('key1', '');
      expect(cache.get('key1')).toBe('');
    });

    it('should handle zero values', () => {
      cache.set('key1', 0);
      expect(cache.get('key1')).toBe(0);
    });

    it('should handle boolean values', () => {
      cache.set('key1', false);
      cache.set('key2', true);
      expect(cache.get('key1')).toBe(false);
      expect(cache.get('key2')).toBe(true);
    });

    it('should handle maximum capacity', () => {
      const maxCache = new QueryCache({ maxSize: 1 });
      maxCache.set('key1', 'value1');
      maxCache.set('key2', 'value2');

      expect(maxCache.has('key1')).toBe(false);
      expect(maxCache.has('key2')).toBe(true);
    });
  });
});
