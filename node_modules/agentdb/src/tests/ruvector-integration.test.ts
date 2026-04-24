/**
 * RuVector Integration Tests
 *
 * Comprehensive test suite for AgentDB's RuVector-powered features:
 * - SIMD Vector Operations
 * - Vector Quantization (8-bit, 4-bit, Product Quantization)
 * - RuVectorBackend Enhancements
 * - Enhanced Embedding Service
 * - Attention Optimized Modules
 *
 * @module ruvector-integration.test
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

// SIMD Vector Operations
import {
  cosineSimilaritySIMD,
  euclideanDistanceSIMD,
  euclideanDistanceSquaredSIMD,
  dotProductSIMD,
  l2NormSIMD,
  batchCosineSimilarity,
  batchEuclideanDistance,
  normalizeVector,
  normalizeVectorInPlace,
  detectSIMDSupport,
  SIMDVectorOps,
  randomUnitVector,
  vectorAdd,
  vectorSub,
  vectorScale,
} from '../simd/simd-vector-ops.js';

// Vector Quantization
import {
  quantize8bit,
  quantize4bit,
  dequantize8bit,
  dequantize4bit,
  calculateQuantizationError,
  getQuantizationStats,
  ProductQuantizer,
  QuantizedVectorStore,
  createScalar8BitStore,
  createScalar4BitStore,
  createProductQuantizedStore,
} from '../quantization/vector-quantization.js';

// Attention Modules
import {
  scaledDotProductAttention,
  scaledDotProductAttentionOptimized,
  batchScaledDotProductAttention,
  batchSequenceAttention,
  MultiHeadAttention,
  MultiHeadAttentionOptimized,
  FlashAttention,
  FlashAttentionOptimized,
  LinearAttention,
  HyperbolicAttention,
  createAttention,
  createAttentionOptimized,
  toFloat32Array,
  flatten2D,
  getBufferPool,
  benchmarkAttention,
} from '../wrappers/attention-fallbacks.js';

// RuVector Backend
import { RuVectorBackend, Semaphore, BufferPool } from '../backends/ruvector/RuVectorBackend.js';

// Enhanced Embedding Service
import { EnhancedEmbeddingService } from '../controllers/EnhancedEmbeddingService.js';

// WASM Vector Search
import { WASMVectorSearch } from '../controllers/WASMVectorSearch.js';

// ============================================================================
// Test Utilities
// ============================================================================

/**
 * Generate a random Float32Array vector
 */
function randomVector(dimension: number): Float32Array {
  const vec = new Float32Array(dimension);
  for (let i = 0; i < dimension; i++) {
    vec[i] = Math.random() * 2 - 1;
  }
  return vec;
}

/**
 * Generate a normalized random vector
 */
function normalizedRandomVector(dimension: number): Float32Array {
  const vec = randomVector(dimension);
  return normalizeVector(vec);
}

/**
 * Compare two Float32Arrays for approximate equality
 */
function vectorsApproxEqual(
  a: Float32Array,
  b: Float32Array,
  tolerance = 1e-5
): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    if (Math.abs(a[i] - b[i]) > tolerance) return false;
  }
  return true;
}

/**
 * Reference cosine similarity implementation for validation
 */
function referenceCosine(a: Float32Array, b: Float32Array): number {
  let dot = 0, normA = 0, normB = 0;
  for (let i = 0; i < a.length; i++) {
    dot += a[i] * b[i];
    normA += a[i] * a[i];
    normB += b[i] * b[i];
  }
  const denom = Math.sqrt(normA) * Math.sqrt(normB);
  return denom === 0 ? 0 : dot / denom;
}

/**
 * Reference Euclidean distance implementation
 */
function referenceEuclidean(a: Float32Array, b: Float32Array): number {
  let sum = 0;
  for (let i = 0; i < a.length; i++) {
    const diff = a[i] - b[i];
    sum += diff * diff;
  }
  return Math.sqrt(sum);
}

// ============================================================================
// 1. SIMD Vector Operations Tests
// ============================================================================

describe('SIMD Vector Operations', () => {
  describe('cosineSimilaritySIMD', () => {
    it('should return 1.0 for identical vectors', () => {
      const v = normalizedRandomVector(384);
      const similarity = cosineSimilaritySIMD(v, v);
      expect(similarity).toBeCloseTo(1.0, 5);
    });

    it('should return -1.0 for opposite vectors', () => {
      const v1 = new Float32Array([1, 0, 0]);
      const v2 = new Float32Array([-1, 0, 0]);
      const similarity = cosineSimilaritySIMD(v1, v2);
      expect(similarity).toBeCloseTo(-1.0, 5);
    });

    it('should return 0.0 for orthogonal vectors', () => {
      const v1 = new Float32Array([1, 0, 0]);
      const v2 = new Float32Array([0, 1, 0]);
      const similarity = cosineSimilaritySIMD(v1, v2);
      expect(similarity).toBeCloseTo(0.0, 5);
    });

    it('should handle zero vectors', () => {
      const zero = new Float32Array([0, 0, 0]);
      const v = new Float32Array([1, 2, 3]);
      const similarity = cosineSimilaritySIMD(zero, v);
      expect(similarity).toBe(0);
    });

    it('should handle empty vectors', () => {
      const empty = new Float32Array(0);
      const similarity = cosineSimilaritySIMD(empty, empty);
      expect(similarity).toBe(0);
    });

    it('should throw error for mismatched dimensions', () => {
      const v1 = new Float32Array([1, 2, 3]);
      const v2 = new Float32Array([1, 2]);
      expect(() => cosineSimilaritySIMD(v1, v2)).toThrow();
    });

    it('should match reference implementation for random vectors', () => {
      for (let i = 0; i < 10; i++) {
        const v1 = randomVector(384);
        const v2 = randomVector(384);
        const simd = cosineSimilaritySIMD(v1, v2);
        const ref = referenceCosine(v1, v2);
        expect(simd).toBeCloseTo(ref, 5);
      }
    });

    it('should handle vectors not divisible by 8', () => {
      const v1 = randomVector(13);
      const v2 = randomVector(13);
      const simd = cosineSimilaritySIMD(v1, v2);
      const ref = referenceCosine(v1, v2);
      expect(simd).toBeCloseTo(ref, 5);
    });

    it('should be bounded between -1 and 1', () => {
      for (let i = 0; i < 100; i++) {
        const v1 = randomVector(384);
        const v2 = randomVector(384);
        const similarity = cosineSimilaritySIMD(v1, v2);
        expect(similarity).toBeGreaterThanOrEqual(-1);
        expect(similarity).toBeLessThanOrEqual(1);
      }
    });
  });

  describe('euclideanDistanceSIMD', () => {
    it('should return 0 for identical vectors', () => {
      const v = randomVector(384);
      const distance = euclideanDistanceSIMD(v, v);
      expect(distance).toBeCloseTo(0, 5);
    });

    it('should compute correct distance for unit vectors', () => {
      const v1 = new Float32Array([1, 0, 0]);
      const v2 = new Float32Array([0, 1, 0]);
      const distance = euclideanDistanceSIMD(v1, v2);
      expect(distance).toBeCloseTo(Math.sqrt(2), 5);
    });

    it('should match reference implementation', () => {
      for (let i = 0; i < 10; i++) {
        const v1 = randomVector(384);
        const v2 = randomVector(384);
        const simd = euclideanDistanceSIMD(v1, v2);
        const ref = referenceEuclidean(v1, v2);
        expect(simd).toBeCloseTo(ref, 5);
      }
    });

    it('should handle empty vectors', () => {
      const empty = new Float32Array(0);
      const distance = euclideanDistanceSIMD(empty, empty);
      expect(distance).toBe(0);
    });

    it('should always be non-negative', () => {
      for (let i = 0; i < 100; i++) {
        const v1 = randomVector(384);
        const v2 = randomVector(384);
        const distance = euclideanDistanceSIMD(v1, v2);
        expect(distance).toBeGreaterThanOrEqual(0);
      }
    });
  });

  describe('batchCosineSimilarity', () => {
    it('should process batch correctly', () => {
      const query = normalizedRandomVector(64);
      const vectors = Array.from({ length: 100 }, () => normalizedRandomVector(64));

      const results = batchCosineSimilarity(query, vectors);

      expect(results).toHaveLength(100);
      expect(results[0].similarity).toBeGreaterThanOrEqual(results[results.length - 1].similarity);
    });

    it('should return top-k results when specified', () => {
      const query = normalizedRandomVector(64);
      const vectors = Array.from({ length: 100 }, () => normalizedRandomVector(64));

      const results = batchCosineSimilarity(query, vectors, { topK: 10 });

      expect(results).toHaveLength(10);
    });

    it('should filter by threshold', () => {
      const query = new Float32Array([1, 0, 0]);
      const vectors = [
        new Float32Array([1, 0, 0]),    // similarity = 1.0
        new Float32Array([0.9, 0.1, 0]), // similarity ~ 0.99
        new Float32Array([0, 1, 0]),     // similarity = 0.0
        new Float32Array([-1, 0, 0]),    // similarity = -1.0
      ];

      const results = batchCosineSimilarity(query, vectors, { threshold: 0.5 });

      expect(results.length).toBe(2);
      results.forEach(r => {
        expect(r.similarity).toBeGreaterThanOrEqual(0.5);
      });
    });

    it('should preserve index information', () => {
      const query = new Float32Array([1, 0, 0]);
      const vectors = [
        new Float32Array([0, 1, 0]),  // index 0
        new Float32Array([1, 0, 0]),  // index 1, most similar
        new Float32Array([0, 0, 1]),  // index 2
      ];

      const results = batchCosineSimilarity(query, vectors, { topK: 1 });

      expect(results[0].index).toBe(1);
    });
  });

  describe('SIMDVectorOps class', () => {
    let ops: SIMDVectorOps;

    beforeEach(() => {
      ops = new SIMDVectorOps({
        bufferPoolSize: 16,
        defaultDimension: 384,
        enableLogging: false,
      });
    });

    afterEach(() => {
      ops.clearBufferPool();
    });

    describe('buffer pool acquire/release', () => {
      it('should acquire buffer of specified size', () => {
        const buffer = ops.acquireBuffer(256);
        expect(buffer).toBeInstanceOf(Float32Array);
        expect(buffer.length).toBe(256);
      });

      it('should reuse released buffers', () => {
        const buffer1 = ops.acquireBuffer(128);
        ops.releaseBuffer(buffer1);
        const buffer2 = ops.acquireBuffer(128);
        // Buffer should be reused (same or similar object)
        expect(buffer2.length).toBe(128);
      });

      it('should track statistics', () => {
        ops.cosineSimilarity(randomVector(64), randomVector(64));
        ops.euclideanDistance(randomVector(64), randomVector(64));

        const stats = ops.getStats();
        expect(stats.operationsCount).toBeGreaterThanOrEqual(2);
        expect(stats.vectorsProcessed).toBeGreaterThanOrEqual(4);
      });

      it('should reset statistics', () => {
        ops.cosineSimilarity(randomVector(64), randomVector(64));
        ops.resetStats();

        const stats = ops.getStats();
        expect(stats.operationsCount).toBe(0);
        expect(stats.vectorsProcessed).toBe(0);
      });
    });

    it('should report SIMD detection status', () => {
      const stats = ops.getStats();
      expect(typeof stats.simdEnabled).toBe('boolean');
    });
  });

  describe('SIMD detection', () => {
    it('should return boolean for SIMD support', () => {
      const supported = detectSIMDSupport();
      expect(typeof supported).toBe('boolean');
    });

    it('should cache detection result', () => {
      const result1 = detectSIMDSupport();
      const result2 = detectSIMDSupport();
      expect(result1).toBe(result2);
    });
  });
});

// ============================================================================
// 2. Vector Quantization Tests
// ============================================================================

describe('Vector Quantization', () => {
  describe('8-bit quantization', () => {
    it('should quantize and dequantize with minimal error', () => {
      const original = new Float32Array([0.1, 0.5, 0.9, -0.3, 0.0]);
      const quantized = quantize8bit(original);

      expect(quantized.type).toBe('8bit');
      expect(quantized.data.length).toBe(original.length);
      expect(quantized.dimension).toBe(original.length);

      const reconstructed = dequantize8bit(
        quantized.data,
        quantized.min,
        quantized.max
      );

      expect(reconstructed.length).toBe(original.length);

      const error = calculateQuantizationError(original, reconstructed);
      expect(error.maxError).toBeLessThan(0.01);
    });

    it('should achieve approximately 4x compression', () => {
      const vector = randomVector(384);
      const stats = getQuantizationStats(vector, '8bit');
      expect(stats.compressionRatio).toBeCloseTo(4, 0.5);
    });

    it('should handle uniform vectors', () => {
      const vector = new Float32Array([0.5, 0.5, 0.5, 0.5]);
      const quantized = quantize8bit(vector);
      const reconstructed = dequantize8bit(
        quantized.data,
        quantized.min,
        quantized.max
      );

      for (let i = 0; i < vector.length; i++) {
        expect(reconstructed[i]).toBeCloseTo(0.5, 5);
      }
    });

    it('should handle empty vectors', () => {
      const empty = new Float32Array(0);
      const quantized = quantize8bit(empty);
      expect(quantized.data.length).toBe(0);

      const reconstructed = dequantize8bit(quantized.data, 0, 0);
      expect(reconstructed.length).toBe(0);
    });

    it('should preserve relative ordering of values', () => {
      const vector = new Float32Array([0.1, 0.3, 0.5, 0.7, 0.9]);
      const quantized = quantize8bit(vector);
      const reconstructed = dequantize8bit(
        quantized.data,
        quantized.min,
        quantized.max
      );

      for (let i = 1; i < reconstructed.length; i++) {
        expect(reconstructed[i]).toBeGreaterThan(reconstructed[i - 1]);
      }
    });
  });

  describe('4-bit quantization', () => {
    it('should quantize and dequantize with acceptable error', () => {
      const original = new Float32Array([0.1, 0.5, 0.9, -0.3, 0.0, 0.7]);
      const quantized = quantize4bit(original);

      expect(quantized.type).toBe('4bit');
      expect(quantized.data.length).toBe(Math.ceil(original.length / 2));
      expect(quantized.dimension).toBe(original.length);

      const reconstructed = dequantize4bit(
        quantized.data,
        quantized.min,
        quantized.max,
        quantized.dimension
      );

      expect(reconstructed.length).toBe(original.length);

      const error = calculateQuantizationError(original, reconstructed);
      expect(error.maxError).toBeLessThan(0.15);
    });

    it('should achieve approximately 8x compression', () => {
      const vector = randomVector(384);
      const stats = getQuantizationStats(vector, '4bit');
      expect(stats.compressionRatio).toBeCloseTo(8, 1);
    });

    it('should handle odd-length vectors', () => {
      const vector = new Float32Array([0.1, 0.5, 0.9]);
      const quantized = quantize4bit(vector);

      expect(quantized.data.length).toBe(2);

      const reconstructed = dequantize4bit(
        quantized.data,
        quantized.min,
        quantized.max,
        quantized.dimension
      );

      expect(reconstructed.length).toBe(3);
    });
  });

  describe('ProductQuantizer', () => {
    const dimension = 64;
    const numSubspaces = 8;
    const numCentroids = 16;

    let pq: ProductQuantizer;
    let trainingVectors: Float32Array[];

    beforeEach(async () => {
      trainingVectors = Array.from({ length: 100 }, () => randomVector(dimension));

      pq = new ProductQuantizer({
        dimension,
        numSubspaces,
        numCentroids,
        maxIterations: 10,
        seed: 42,
      });

      await pq.train(trainingVectors);
    });

    it('should train successfully', () => {
      expect(pq.isTrained()).toBe(true);
    });

    it('should encode and decode vectors', () => {
      const vector = trainingVectors[0];
      const encoded = pq.encode(vector);

      expect(encoded.codes.length).toBe(numSubspaces);
      expect(encoded.norm).toBeGreaterThan(0);

      const decoded = pq.decode(encoded);
      expect(decoded.length).toBe(dimension);
    });

    it('should compute asymmetric distance', () => {
      const query = trainingVectors[0];
      const encoded = pq.encode(trainingVectors[1]);

      const distance = pq.asymmetricDistance(query, encoded);
      expect(distance).toBeGreaterThanOrEqual(0);
    });

    it('should compute distance using precomputed tables', () => {
      const query = trainingVectors[0];
      const encoded = pq.encode(trainingVectors[1]);

      const tables = pq.precomputeDistanceTables(query);
      expect(tables.length).toBe(numSubspaces);

      const tableDistance = pq.distanceFromTables(tables, encoded);
      const directDistance = pq.asymmetricDistance(query, encoded);

      expect(tableDistance).toBeCloseTo(directDistance, 5);
    });

    it('should export and import codebooks', () => {
      const exported = pq.exportCodebooks();
      expect(typeof exported).toBe('string');

      const pq2 = new ProductQuantizer({
        dimension,
        numSubspaces,
        numCentroids,
      });

      pq2.importCodebooks(exported);
      expect(pq2.isTrained()).toBe(true);

      const vector = trainingVectors[0];
      const encoded1 = pq.encode(vector);
      const encoded2 = pq2.encode(vector);

      expect(Array.from(encoded1.codes)).toEqual(Array.from(encoded2.codes));
    });

    it('should report correct stats', () => {
      const stats = pq.getStats();

      expect(stats.trained).toBe(true);
      expect(stats.dimension).toBe(dimension);
      expect(stats.numSubspaces).toBe(numSubspaces);
      expect(stats.subspaceDim).toBe(dimension / numSubspaces);
      expect(stats.numCentroids).toBe(numCentroids);
      expect(stats.compressionRatio).toBeGreaterThan(1);
      expect(stats.codebookSizeBytes).toBeGreaterThan(0);
    });

    it('should throw on invalid dimension', () => {
      expect(() => {
        new ProductQuantizer({
          dimension: 65,
          numSubspaces: 8,
          numCentroids: 256,
        });
      }).toThrow();
    });

    it('should throw on too many centroids', () => {
      expect(() => {
        new ProductQuantizer({
          dimension: 64,
          numSubspaces: 8,
          numCentroids: 257,
        });
      }).toThrow();
    });
  });

  describe('QuantizedVectorStore', () => {
    const dimension = 64;

    describe('Scalar 8-bit store', () => {
      let store: QuantizedVectorStore;

      beforeEach(() => {
        store = createScalar8BitStore(dimension);
      });

      it('should insert and search vectors', () => {
        const v1 = normalizedRandomVector(dimension);
        const v2 = normalizedRandomVector(dimension);
        const v3 = normalizedRandomVector(dimension);

        store.insert('v1', v1, { label: 'first' });
        store.insert('v2', v2, { label: 'second' });
        store.insert('v3', v3, { label: 'third' });

        const results = store.search(v1, 3);

        expect(results.length).toBe(3);
        expect(results[0].id).toBe('v1');
        expect(results[0].similarity).toBeGreaterThan(0.9);
        expect(results[0].metadata?.label).toBe('first');
      });

      it('should remove vectors', () => {
        store.insert('v1', randomVector(dimension));
        store.insert('v2', randomVector(dimension));

        expect(store.remove('v1')).toBe(true);
        expect(store.remove('v1')).toBe(false);

        const stats = store.getStats();
        expect(stats.count).toBe(1);
      });

      it('should retrieve dequantized vectors', () => {
        const original = randomVector(dimension);
        store.insert('v1', original);

        const retrieved = store.getVector('v1');
        expect(retrieved).not.toBeNull();
        expect(retrieved!.length).toBe(dimension);

        const error = calculateQuantizationError(original, retrieved!);
        expect(error.meanError).toBeLessThan(0.05);
      });

      it('should export and import store', () => {
        store.insert('v1', randomVector(dimension), { tag: 'a' });
        store.insert('v2', randomVector(dimension), { tag: 'b' });

        const exported = store.export();

        const store2 = createScalar8BitStore(dimension);
        store2.import(exported);

        const stats = store2.getStats();
        expect(stats.count).toBe(2);

        const v1 = store2.getVector('v1');
        expect(v1).not.toBeNull();
      });

      it('should batch insert vectors', () => {
        const items = Array.from({ length: 50 }, (_, i) => ({
          id: `v${i}`,
          vector: randomVector(dimension),
          metadata: { index: i },
        }));

        store.insertBatch(items);

        const stats = store.getStats();
        expect(stats.count).toBe(50);
      });

      it('should report memory usage', () => {
        for (let i = 0; i < 100; i++) {
          store.insert(`v${i}`, randomVector(dimension));
        }

        const stats = store.getStats();
        expect(stats.memoryUsageBytes).toBeGreaterThan(0);
        expect(stats.memoryUsageBytes).toBeLessThan(dimension * 4 * 100);
      });
    });

    describe('Scalar 4-bit store', () => {
      it('should achieve higher compression than 8-bit', () => {
        const store4bit = createScalar4BitStore(dimension);
        const store8bit = createScalar8BitStore(dimension);

        for (let i = 0; i < 100; i++) {
          const vec = randomVector(dimension);
          store4bit.insert(`v${i}`, vec);
          store8bit.insert(`v${i}`, vec);
        }

        const stats4bit = store4bit.getStats();
        const stats8bit = store8bit.getStats();

        expect(stats4bit.compressionRatio).toBeGreaterThan(stats8bit.compressionRatio);
      });

      it('should still find similar vectors', () => {
        const store = createScalar4BitStore(dimension);

        const baseVector = normalizedRandomVector(dimension);
        const similarVector = new Float32Array(baseVector);
        for (let i = 0; i < dimension; i++) {
          similarVector[i] += (Math.random() - 0.5) * 0.1;
        }

        store.insert('base', baseVector);
        store.insert('similar', normalizeVector(similarVector));
        store.insert('random', normalizedRandomVector(dimension));

        const results = store.search(baseVector, 2);

        expect(results.length).toBe(2);
        expect(results[0].id).toBe('base');
        expect(results[1].id).toBe('similar');
      });
    });

    describe('Product Quantized store', () => {
      it('should require training before insertion', () => {
        const store = createProductQuantizedStore(dimension, 8, 16);

        expect(store.isReady()).toBe(false);

        expect(() => {
          store.insert('v1', randomVector(dimension));
        }).toThrow();
      });

      it('should work after training', async () => {
        const store = createProductQuantizedStore(dimension, 8, 16);

        const trainingData = Array.from({ length: 50 }, () =>
          normalizedRandomVector(dimension)
        );

        await store.train(trainingData);
        expect(store.isReady()).toBe(true);

        store.insert('v1', trainingData[0], { index: 0 });
        store.insert('v2', trainingData[1], { index: 1 });

        const results = store.search(trainingData[0], 2);
        expect(results.length).toBe(2);
        expect(results[0].id).toBe('v1');
      });

      it('should achieve high compression', async () => {
        const store = createProductQuantizedStore(dimension, 8, 16);

        const trainingData = Array.from({ length: 50 }, () => randomVector(dimension));

        await store.train(trainingData);

        for (let i = 0; i < 50; i++) {
          store.insert(`v${i}`, trainingData[i]);
        }

        const stats = store.getStats();
        expect(stats.compressionRatio).toBeGreaterThan(4);
      });
    });
  });

  describe('Quantization accuracy measurement', () => {
    it('should track mean and max error', () => {
      const original = randomVector(384);
      const quantized = quantize8bit(original);
      const reconstructed = dequantize8bit(
        quantized.data,
        quantized.min,
        quantized.max
      );

      const error = calculateQuantizationError(original, reconstructed);

      expect(error.meanError).toBeGreaterThanOrEqual(0);
      expect(error.maxError).toBeGreaterThanOrEqual(error.meanError);
      expect(error.mse).toBeGreaterThanOrEqual(0);
    });

    it('should throw for mismatched lengths', () => {
      const v1 = new Float32Array([1, 2, 3]);
      const v2 = new Float32Array([1, 2]);

      expect(() => calculateQuantizationError(v1, v2)).toThrow();
    });
  });
});

// ============================================================================
// 3. RuVectorBackend Enhancement Tests
// ============================================================================

describe('RuVectorBackend Enhancements', () => {
  describe('Semaphore', () => {
    it('should limit concurrent operations', async () => {
      const semaphore = new Semaphore(2);
      let concurrent = 0;
      let maxConcurrent = 0;

      const tasks = Array.from({ length: 10 }, async () => {
        await semaphore.acquire();
        concurrent++;
        maxConcurrent = Math.max(maxConcurrent, concurrent);

        await new Promise(resolve => setTimeout(resolve, 10));

        concurrent--;
        semaphore.release();
      });

      await Promise.all(tasks);

      expect(maxConcurrent).toBeLessThanOrEqual(2);
    });

    it('should track available permits', () => {
      const semaphore = new Semaphore(3);

      expect(semaphore.available).toBe(3);

      semaphore.acquire();
      expect(semaphore.available).toBe(2);

      semaphore.release();
      expect(semaphore.available).toBe(3);
    });
  });

  describe('BufferPool', () => {
    let pool: BufferPool;

    beforeEach(() => {
      pool = new BufferPool(10);
    });

    afterEach(() => {
      pool.clear();
    });

    it('should acquire new buffers', () => {
      const buffer = pool.acquire(256);
      expect(buffer).toBeInstanceOf(Float32Array);
      expect(buffer.length).toBe(256);
    });

    it('should reuse released buffers', () => {
      const buffer1 = pool.acquire(128);
      buffer1[0] = 42;

      pool.release(buffer1);

      const buffer2 = pool.acquire(128);
      expect(buffer2[0]).toBe(0); // Buffer should be cleared
    });

    it('should maintain separate pools for different sizes', () => {
      const buf128 = pool.acquire(128);
      const buf256 = pool.acquire(256);

      pool.release(buf128);
      pool.release(buf256);

      const stats = pool.getStats();
      expect(stats.totalBuffers).toBe(2);
    });

    it('should report statistics', () => {
      pool.acquire(64);
      pool.acquire(64);
      const buf = pool.acquire(128);
      pool.release(buf);

      const stats = pool.getStats();
      expect(stats.totalBuffers).toBeGreaterThanOrEqual(1);
      expect(stats.totalMemory).toBeGreaterThanOrEqual(128 * 4);
    });
  });

  describe('Adaptive parameters', () => {
    it('should return small dataset params for < 1000 vectors', () => {
      const params = RuVectorBackend.getRecommendedParams(500);
      expect(params.M).toBe(8);
      expect(params.efConstruction).toBe(100);
      expect(params.efSearch).toBe(50);
    });

    it('should return medium dataset params for 1000-100000 vectors', () => {
      const params = RuVectorBackend.getRecommendedParams(50000);
      expect(params.M).toBe(16);
      expect(params.efConstruction).toBe(200);
      expect(params.efSearch).toBe(100);
    });

    it('should return large dataset params for > 100000 vectors', () => {
      const params = RuVectorBackend.getRecommendedParams(500000);
      expect(params.M).toBe(32);
      expect(params.efConstruction).toBe(400);
      expect(params.efSearch).toBe(200);
    });
  });
});

// ============================================================================
// 4. Enhanced Embedding Service Tests
// ============================================================================

describe('EnhancedEmbeddingService', () => {
  let service: EnhancedEmbeddingService;

  beforeEach(async () => {
    service = new EnhancedEmbeddingService({
      model: 'mock-model',
      dimension: 384,
      provider: 'local',
      enableWASM: true,
      enableBatchProcessing: true,
      batchSize: 50,
    });

    await service.initialize();
  });

  describe('Batch embedding', () => {
    it('should embed batch of texts', async () => {
      const texts = ['hello', 'world', 'test', 'embedding', 'service'];
      const embeddings = await service.embedBatch(texts);

      expect(embeddings).toHaveLength(5);
      embeddings.forEach(emb => {
        expect(emb).toBeInstanceOf(Float32Array);
        expect(emb.length).toBe(384);
      });
    });

    it('should handle large batches', async () => {
      const texts = Array.from({ length: 200 }, (_, i) => `text ${i}`);
      const embeddings = await service.embedBatch(texts);

      expect(embeddings).toHaveLength(200);
    });

    it('should deduplicate identical texts', async () => {
      const texts = ['same', 'same', 'same', 'different'];
      const embeddings = await service.embedBatch(texts);

      expect(embeddings).toHaveLength(4);
      expect(vectorsApproxEqual(embeddings[0], embeddings[1])).toBe(true);
      expect(vectorsApproxEqual(embeddings[0], embeddings[2])).toBe(true);
    });
  });

  describe('Similarity calculation', () => {
    it('should calculate text similarity', async () => {
      const similarity = await service.similarity('hello world', 'hello world');
      expect(similarity).toBeCloseTo(1.0, 5);
    });

    it('should find most similar texts', async () => {
      const corpus = [
        'machine learning',
        'artificial intelligence',
        'deep learning',
        'cooking recipes',
        'neural networks',
      ];

      const results = await service.findMostSimilar('AI and ML', corpus, 3);

      expect(results).toHaveLength(3);
      expect(results[0]).toHaveProperty('text');
      expect(results[0]).toHaveProperty('similarity');
      expect(results[0]).toHaveProperty('index');

      results.forEach(result => {
        expect(result.similarity).toBeGreaterThanOrEqual(-1);
        expect(result.similarity).toBeLessThanOrEqual(1);
        expect(corpus).toContain(result.text);
      });
    });
  });

  describe('Statistics', () => {
    it('should provide service statistics', () => {
      const stats = service.getStats();

      expect(stats).toHaveProperty('cacheSize');
      expect(stats).toHaveProperty('wasmEnabled');
      expect(stats).toHaveProperty('simdEnabled');
      expect(typeof stats.cacheSize).toBe('number');
    });
  });

  describe('LRU cache behavior', () => {
    it('should cache embeddings', async () => {
      await service.embed('cached text');
      await service.embed('cached text');

      const stats = service.getStats();
      expect(stats.cacheSize).toBeGreaterThanOrEqual(1);
    });
  });
});

// ============================================================================
// 5. Attention Optimized Tests
// ============================================================================

describe('Attention Optimized', () => {
  describe('scaledDotProductAttention', () => {
    it('should compute attention correctly', () => {
      const query = [1, 0, 0, 0];
      const key = [1, 0, 0, 0];
      const value = [1, 2, 3, 4];

      const { output, weights } = scaledDotProductAttention(query, key, value);

      expect(output).toHaveLength(4);
      expect(weights).toHaveLength(1);
    });

    it('should apply mask', () => {
      const query = [1, 0, 0, 0];
      const key = [1, 0, 0, 0];
      const value = [1, 2, 3, 4];
      const mask = [0];

      const { weights } = scaledDotProductAttention(query, key, value, mask);

      expect(weights[0]).toBe(0);
    });
  });

  describe('scaledDotProductAttentionOptimized', () => {
    it('should compute attention with TypedArrays', () => {
      const query = new Float32Array([1, 0, 0, 0]);
      const key = new Float32Array([1, 0, 0, 0]);
      const value = new Float32Array([1, 2, 3, 4]);

      const { output, weights } = scaledDotProductAttentionOptimized(query, key, value);

      expect(output).toBeInstanceOf(Float32Array);
      expect(output.length).toBe(4);
      expect(weights).toBeInstanceOf(Float32Array);
    });

    it('should match original implementation', () => {
      const query = new Float32Array([0.5, 0.3, 0.2, 0.1]);
      const key = new Float32Array([0.4, 0.3, 0.2, 0.1]);
      const value = new Float32Array([1, 2, 3, 4]);

      const optimized = scaledDotProductAttentionOptimized(query, key, value);
      const original = scaledDotProductAttention(
        Array.from(query),
        Array.from(key),
        Array.from(value)
      );

      for (let i = 0; i < value.length; i++) {
        expect(optimized.output[i]).toBeCloseTo(original.output[i], 4);
      }
    });
  });

  describe('MultiHeadAttention vs MultiHeadAttentionOptimized', () => {
    const config = { hiddenDim: 64, numHeads: 8 };

    it('should produce outputs of same dimensions', () => {
      const mha = new MultiHeadAttention(config);
      const mhaOpt = new MultiHeadAttentionOptimized(config);

      const query = Array.from({ length: 64 }, () => Math.random());
      const key = Array.from({ length: 64 }, () => Math.random());
      const value = Array.from({ length: 64 }, () => Math.random());

      const queryF32 = new Float32Array(query);
      const keyF32 = new Float32Array(key);
      const valueF32 = new Float32Array(value);

      const original = mha.forward(query, key, value);
      const optimized = mhaOpt.forward(queryF32, keyF32, valueF32);

      expect(original.output.length).toBe(optimized.output.length);
      expect(original.attentionWeights.length).toBe(optimized.attentionWeights.length);
    });

    it('should support batch processing', () => {
      const mhaOpt = new MultiHeadAttentionOptimized(config);

      const batchSize = 4;
      const queries = Array.from({ length: batchSize }, () => randomVector(64));
      const keys = Array.from({ length: batchSize }, () => randomVector(64));
      const values = Array.from({ length: batchSize }, () => randomVector(64));

      const { outputs, attentionWeights } = mhaOpt.batchForward(queries, keys, values);

      expect(outputs).toHaveLength(batchSize);
      expect(attentionWeights).toHaveLength(batchSize);
    });

    it('should allow weight get/set', () => {
      const mhaOpt = new MultiHeadAttentionOptimized(config);

      const weights = mhaOpt.getWeights();
      expect(weights.query).toBeInstanceOf(Float32Array);
      expect(weights.key).toBeInstanceOf(Float32Array);
      expect(weights.value).toBeInstanceOf(Float32Array);
      expect(weights.output).toBeInstanceOf(Float32Array);

      // Modify and set back
      weights.query[0] = 999;
      mhaOpt.setWeights(weights);

      const retrieved = mhaOpt.getWeights();
      expect(retrieved.query[0]).toBe(999);
    });
  });

  describe('FlashAttention vs FlashAttentionOptimized', () => {
    const config = { hiddenDim: 64 };

    it('should compute attention over sequences', () => {
      const flash = new FlashAttention(config);

      const seqLen = 4;
      const query = Array.from({ length: seqLen }, () =>
        Array.from({ length: 64 }, () => Math.random())
      );
      const key = Array.from({ length: seqLen }, () =>
        Array.from({ length: 64 }, () => Math.random())
      );
      const value = Array.from({ length: seqLen }, () =>
        Array.from({ length: 64 }, () => Math.random())
      );

      const { output, attentionScores } = flash.forward(query, key, value, 8);

      expect(output).toHaveLength(seqLen);
      expect(attentionScores).toHaveLength(seqLen);
    });

    it('should match dimensions in optimized version', () => {
      const flashOpt = new FlashAttentionOptimized({
        hiddenDim: 64,
        blockSizeQ: 2,
        blockSizeKV: 4,
      });

      const seqLen = 8;
      const dim = 64;
      const query = randomVector(seqLen * dim);
      const key = randomVector(seqLen * dim);
      const value = randomVector(seqLen * dim);

      const { output, attentionScores } = flashOpt.forward(
        query,
        key,
        value,
        seqLen,
        dim,
        8
      );

      expect(output.length).toBe(seqLen * dim);
      expect(attentionScores.length).toBe(seqLen * seqLen);

      flashOpt.releaseBuffer(output);
      flashOpt.releaseBuffer(attentionScores);
    });

    it('should support causal masking', () => {
      const flashOpt = new FlashAttentionOptimized({
        hiddenDim: 64,
        causal: true,
      });

      const seqLen = 4;
      const dim = 64;
      const query = randomVector(seqLen * dim);
      const key = randomVector(seqLen * dim);
      const value = randomVector(seqLen * dim);

      const { attentionScores } = flashOpt.forward(
        query,
        key,
        value,
        seqLen,
        dim,
        8
      );

      // The implementation stores raw scores (pre-softmax), so masked positions
      // should have very large negative values (approximately -1e9)
      // Check upper triangle has been masked (large negative values)
      for (let i = 0; i < seqLen; i++) {
        for (let j = i + 1; j < seqLen; j++) {
          // Masked positions should be -1e9 (or very close to it)
          expect(attentionScores[i * seqLen + j]).toBeLessThan(-1e8);
        }
      }

      // Also verify lower triangle (including diagonal) is NOT masked
      // These should be normal attention scores, not large negative values
      for (let i = 0; i < seqLen; i++) {
        for (let j = 0; j <= i; j++) {
          expect(attentionScores[i * seqLen + j]).toBeGreaterThan(-100);
        }
      }
    });

    it('should support batch processing', () => {
      const flashOpt = new FlashAttentionOptimized({ hiddenDim: 32 });

      const batchSize = 3;
      const seqLens = [4, 4, 4];
      const dim = 32;

      const queries = seqLens.map(len => randomVector(len * dim));
      const keys = seqLens.map(len => randomVector(len * dim));
      const values = seqLens.map(len => randomVector(len * dim));

      const { outputs, attentionScores } = flashOpt.batchForward(
        queries,
        keys,
        values,
        seqLens,
        dim,
        4
      );

      expect(outputs).toHaveLength(batchSize);
      expect(attentionScores).toHaveLength(batchSize);
    });
  });

  describe('batchSequenceAttention', () => {
    it('should process sequence attention', () => {
      const seqLen = 4;
      const dim = 8;

      const queries = randomVector(seqLen * dim);
      const keys = randomVector(seqLen * dim);
      const values = randomVector(seqLen * dim);

      const { output, weights } = batchSequenceAttention(
        queries,
        keys,
        values,
        seqLen,
        dim
      );

      expect(output.length).toBe(seqLen * dim);
      expect(weights.length).toBe(seqLen * seqLen);
    });

    it('should apply mask correctly', () => {
      const seqLen = 2;
      const dim = 4;

      const queries = new Float32Array([1, 0, 0, 0, 0, 1, 0, 0]);
      const keys = new Float32Array([1, 0, 0, 0, 0, 1, 0, 0]);
      const values = new Float32Array([1, 2, 3, 4, 5, 6, 7, 8]);
      const mask = new Float32Array([1, 0, 1, 1]); // Block attention from pos 0 to pos 1

      const { weights } = batchSequenceAttention(
        queries,
        keys,
        values,
        seqLen,
        dim,
        mask
      );

      // Weight from position 0 to position 1 should be very small
      expect(weights[1]).toBeLessThan(0.01);
    });
  });

  describe('LinearAttention', () => {
    it('should compute linear attention', () => {
      const linear = new LinearAttention({ hiddenDim: 8 });

      const query = [[1, 2, 3, 4, 5, 6, 7, 8]];
      const key = [[1, 2, 3, 4, 5, 6, 7, 8]];
      const value = [[1, 2, 3, 4, 5, 6, 7, 8]];

      const { output } = linear.forward(query, key, value);

      expect(output).toHaveLength(1);
      expect(output[0]).toHaveLength(8);
    });
  });

  describe('HyperbolicAttention', () => {
    it('should compute hyperbolic attention', () => {
      const hyper = new HyperbolicAttention({ hiddenDim: 4 });

      const query = [0.1, 0.2, 0.3, 0.4];
      const key = [0.15, 0.25, 0.35, 0.45];
      const value = [1, 2, 3, 4];

      const { output, distance } = hyper.forward(query, key, value);

      expect(output).toHaveLength(4);
      expect(distance).toBeGreaterThanOrEqual(0);
    });
  });

  describe('Factory functions', () => {
    it('should create attention modules', () => {
      const config = { hiddenDim: 64 };

      const mha = createAttention('multi-head', config);
      expect(mha).toBeInstanceOf(MultiHeadAttention);

      const flash = createAttention('flash', config);
      expect(flash).toBeInstanceOf(FlashAttention);

      const linear = createAttention('linear', config);
      expect(linear).toBeInstanceOf(LinearAttention);

      const hyper = createAttention('hyperbolic', config);
      expect(hyper).toBeInstanceOf(HyperbolicAttention);
    });

    it('should create optimized attention modules', () => {
      const config = { hiddenDim: 64 };

      const mhaOpt = createAttentionOptimized('multi-head', config);
      expect(mhaOpt).toBeInstanceOf(MultiHeadAttentionOptimized);

      const flashOpt = createAttentionOptimized('flash', config);
      expect(flashOpt).toBeInstanceOf(FlashAttentionOptimized);
    });
  });

  describe('Buffer pool', () => {
    it('should provide global buffer pool access', () => {
      const pool = getBufferPool();

      const buffer = pool.acquire(64);
      expect(buffer).toBeInstanceOf(Float32Array);
      expect(buffer.length).toBe(64);

      pool.release(buffer);
    });
  });

  describe('Utility functions', () => {
    it('should convert arrays to Float32Array', () => {
      const arr = [1, 2, 3, 4];
      const f32 = toFloat32Array(arr);

      expect(f32).toBeInstanceOf(Float32Array);
      expect(Array.from(f32)).toEqual(arr);
    });

    it('should flatten 2D arrays', () => {
      const arr2d = [[1, 2], [3, 4], [5, 6]];
      const flat = flatten2D(arr2d);

      expect(flat).toBeInstanceOf(Float32Array);
      expect(Array.from(flat)).toEqual([1, 2, 3, 4, 5, 6]);
    });
  });
});

// ============================================================================
// 6. WASM Vector Search Tests
// ============================================================================

describe('WASMVectorSearch', () => {
  let mockDb: any;
  let wasmSearch: WASMVectorSearch;

  beforeEach(() => {
    mockDb = {
      prepare: () => ({
        all: () => [],
        get: () => null,
        run: () => ({ lastInsertRowid: 1, changes: 1 }),
      }),
      exec: () => {},
    };

    wasmSearch = new WASMVectorSearch(mockDb);
  });

  describe('Cosine Similarity', () => {
    it('should calculate correctly for identical vectors', () => {
      const v = new Float32Array([1, 0, 0]);
      const similarity = wasmSearch.cosineSimilarity(v, v);
      expect(similarity).toBeCloseTo(1.0, 5);
    });

    it('should handle orthogonal vectors', () => {
      const v1 = new Float32Array([1, 0, 0]);
      const v2 = new Float32Array([0, 1, 0]);
      const similarity = wasmSearch.cosineSimilarity(v1, v2);
      expect(similarity).toBeCloseTo(0.0, 5);
    });

    it('should throw on mismatched dimensions', () => {
      const v1 = new Float32Array([1, 0, 0]);
      const v2 = new Float32Array([1, 0]);
      expect(() => wasmSearch.cosineSimilarity(v1, v2)).toThrow();
    });
  });

  describe('Batch Operations', () => {
    it('should calculate batch similarities', () => {
      const query = new Float32Array([1, 0, 0]);
      const vectors = [
        new Float32Array([1, 0, 0]),
        new Float32Array([0, 1, 0]),
        new Float32Array([0, 0, 1]),
      ];

      const similarities = wasmSearch.batchSimilarity(query, vectors);

      expect(similarities).toHaveLength(3);
      expect(similarities[0]).toBeCloseTo(1.0, 5);
      expect(similarities[1]).toBeCloseTo(0.0, 5);
      expect(similarities[2]).toBeCloseTo(0.0, 5);
    });

    it('should handle large batches', () => {
      const query = new Float32Array(384).fill(0.5);
      const vectors: Float32Array[] = [];

      for (let i = 0; i < 1000; i++) {
        vectors.push(new Float32Array(384).fill(Math.random()));
      }

      const startTime = performance.now();
      const similarities = wasmSearch.batchSimilarity(query, vectors);
      const duration = performance.now() - startTime;

      expect(similarities).toHaveLength(1000);
      expect(duration).toBeLessThan(1000);
    });
  });

  describe('Vector Index', () => {
    it('should build index for large datasets', () => {
      const vectors: Float32Array[] = [];
      const ids: number[] = [];

      for (let i = 0; i < 1500; i++) {
        vectors.push(new Float32Array(128).fill(Math.random()));
        ids.push(i);
      }

      wasmSearch.buildIndex(vectors, ids);

      const stats = wasmSearch.getStats();
      expect(stats.indexBuilt).toBe(true);
      expect(stats.indexSize).toBe(1500);
    });

    it('should skip index for small datasets', () => {
      const vectors = [new Float32Array([1, 0, 0])];
      const ids = [1];

      wasmSearch.buildIndex(vectors, ids);

      const stats = wasmSearch.getStats();
      expect(stats.indexBuilt).toBe(false);
    });

    it('should search index correctly', () => {
      const vectors: Float32Array[] = [
        new Float32Array([1, 0, 0]),
        new Float32Array([0, 1, 0]),
        new Float32Array([0, 0, 1]),
        new Float32Array([0.7, 0.7, 0]),
      ];
      const ids = [1, 2, 3, 4];

      wasmSearch = new WASMVectorSearch(mockDb, { indexThreshold: 3 });
      wasmSearch.buildIndex(vectors, ids);

      const query = new Float32Array([1, 0, 0]);
      const results = wasmSearch.searchIndex(query, 2);

      expect(results).toHaveLength(2);
      expect(results[0].id).toBe(1);
      expect(results[0].similarity).toBeCloseTo(1.0, 5);
    });

    it('should clear index', () => {
      wasmSearch = new WASMVectorSearch(mockDb, { indexThreshold: 0 });
      wasmSearch.buildIndex([new Float32Array([1, 0, 0])], [1]);

      let stats = wasmSearch.getStats();
      expect(stats.indexBuilt).toBe(true);

      wasmSearch.clearIndex();
      stats = wasmSearch.getStats();
      expect(stats.indexBuilt).toBe(false);
    });
  });

  describe('Statistics', () => {
    it('should report correct stats', () => {
      const stats = wasmSearch.getStats();

      expect(stats).toHaveProperty('wasmAvailable');
      expect(stats).toHaveProperty('simdAvailable');
      expect(stats).toHaveProperty('indexBuilt');
      expect(stats).toHaveProperty('indexSize');
      expect(typeof stats.wasmAvailable).toBe('boolean');
      expect(typeof stats.simdAvailable).toBe('boolean');
    });
  });
});

// ============================================================================
// Performance Tests
// ============================================================================

describe('Performance', () => {
  describe('SIMD operations performance', () => {
    it('should process 1000 similarity calculations efficiently', () => {
      const query = randomVector(384);
      const vectors = Array.from({ length: 1000 }, () => randomVector(384));

      const startTime = performance.now();
      vectors.forEach(v => cosineSimilaritySIMD(query, v));
      const duration = performance.now() - startTime;

      expect(duration).toBeLessThan(500);
    });

    it('should batch process efficiently', () => {
      const query = randomVector(384);
      const vectors = Array.from({ length: 10000 }, () => randomVector(384));

      const startTime = performance.now();
      batchCosineSimilarity(query, vectors, { topK: 100 });
      const duration = performance.now() - startTime;

      expect(duration).toBeLessThan(5000);
    });
  });

  describe('Quantization performance', () => {
    it('should quantize 1000 vectors efficiently', () => {
      const vectors = Array.from({ length: 1000 }, () => randomVector(384));

      const startTime = performance.now();
      vectors.forEach(v => quantize8bit(v));
      const duration = performance.now() - startTime;

      expect(duration).toBeLessThan(1000);
    });
  });

  describe('Attention performance', () => {
    it('should benchmark attention implementation', () => {
      const attention = new FlashAttentionOptimized({ hiddenDim: 64 });

      const result = benchmarkAttention(attention, {
        seqLen: 32,
        dim: 64,
        iterations: 10,
        numHeads: 8,
      });

      expect(result.avgTimeMs).toBeGreaterThan(0);
      expect(result.opsPerSecond).toBeGreaterThan(0);
      expect(result.memoryUsed).toBeGreaterThan(0);
    });
  });
});
