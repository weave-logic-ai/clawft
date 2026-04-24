/**
 * Tests for Vector Quantization Module
 */

import { describe, it, expect, beforeAll } from 'vitest';
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

describe('Scalar Quantization', () => {
  describe('8-bit quantization', () => {
    it('should quantize and dequantize with minimal error', () => {
      const vector = new Float32Array([0.1, 0.5, 0.9, -0.3, 0.0]);
      const quantized = quantize8bit(vector);

      expect(quantized.type).toBe('8bit');
      expect(quantized.data.length).toBe(vector.length);
      expect(quantized.dimension).toBe(vector.length);

      const reconstructed = dequantize8bit(
        quantized.data,
        quantized.min,
        quantized.max
      );

      expect(reconstructed.length).toBe(vector.length);

      // Check reconstruction error is small
      const error = calculateQuantizationError(vector, reconstructed);
      expect(error.maxError).toBeLessThan(0.01); // 8-bit should be very accurate
    });

    it('should achieve 4x compression ratio', () => {
      const vector = new Float32Array(384);
      for (let i = 0; i < 384; i++) {
        vector[i] = Math.random() * 2 - 1;
      }

      const stats = getQuantizationStats(vector, '8bit');
      expect(stats.compressionRatio).toBeCloseTo(4, 1);
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
      const vector = new Float32Array(0);
      const quantized = quantize8bit(vector);
      expect(quantized.data.length).toBe(0);

      const reconstructed = dequantize8bit(quantized.data, 0, 0);
      expect(reconstructed.length).toBe(0);
    });
  });

  describe('4-bit quantization', () => {
    it('should quantize and dequantize with acceptable error', () => {
      const vector = new Float32Array([0.1, 0.5, 0.9, -0.3, 0.0, 0.7]);
      const quantized = quantize4bit(vector);

      expect(quantized.type).toBe('4bit');
      expect(quantized.data.length).toBe(Math.ceil(vector.length / 2));
      expect(quantized.dimension).toBe(vector.length);

      const reconstructed = dequantize4bit(
        quantized.data,
        quantized.min,
        quantized.max,
        quantized.dimension
      );

      expect(reconstructed.length).toBe(vector.length);

      // 4-bit has larger quantization error
      const error = calculateQuantizationError(vector, reconstructed);
      expect(error.maxError).toBeLessThan(0.1);
    });

    it('should achieve 8x compression ratio', () => {
      const vector = new Float32Array(384);
      for (let i = 0; i < 384; i++) {
        vector[i] = Math.random() * 2 - 1;
      }

      const stats = getQuantizationStats(vector, '4bit');
      expect(stats.compressionRatio).toBeCloseTo(8, 1);
    });

    it('should handle odd-length vectors', () => {
      const vector = new Float32Array([0.1, 0.5, 0.9]);
      const quantized = quantize4bit(vector);

      expect(quantized.data.length).toBe(2); // ceil(3/2) = 2

      const reconstructed = dequantize4bit(
        quantized.data,
        quantized.min,
        quantized.max,
        quantized.dimension
      );

      expect(reconstructed.length).toBe(3);
    });
  });
});

describe('ProductQuantizer', () => {
  const dimension = 64;
  const numSubspaces = 8;
  const numCentroids = 16; // Smaller for faster tests

  let pq: ProductQuantizer;
  let trainingVectors: Float32Array[];

  beforeAll(async () => {
    // Generate training data
    trainingVectors = [];
    for (let i = 0; i < 100; i++) {
      const vec = new Float32Array(dimension);
      for (let j = 0; j < dimension; j++) {
        vec[j] = Math.random() * 2 - 1;
      }
      trainingVectors.push(vec);
    }

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

    const distance = pq.distanceFromTables(tables, encoded);
    const directDistance = pq.asymmetricDistance(query, encoded);

    expect(distance).toBeCloseTo(directDistance, 5);
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

    // Verify encoding produces same results
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
        dimension: 65, // Not divisible by 8
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
        numCentroids: 257, // Max is 256
      });
    }).toThrow();
  });
});

describe('QuantizedVectorStore', () => {
  const dimension = 64;

  function generateRandomVector(): Float32Array {
    const vec = new Float32Array(dimension);
    for (let i = 0; i < dimension; i++) {
      vec[i] = Math.random() * 2 - 1;
    }
    return vec;
  }

  function normalizeVector(vec: Float32Array): Float32Array {
    let norm = 0;
    for (let i = 0; i < vec.length; i++) {
      norm += vec[i] * vec[i];
    }
    norm = Math.sqrt(norm);

    const result = new Float32Array(vec.length);
    for (let i = 0; i < vec.length; i++) {
      result[i] = vec[i] / norm;
    }
    return result;
  }

  describe('Scalar 8-bit store', () => {
    it('should insert and search vectors', () => {
      const store = createScalar8BitStore(dimension);

      const v1 = normalizeVector(generateRandomVector());
      const v2 = normalizeVector(generateRandomVector());
      const v3 = normalizeVector(generateRandomVector());

      store.insert('v1', v1, { label: 'first' });
      store.insert('v2', v2, { label: 'second' });
      store.insert('v3', v3, { label: 'third' });

      const results = store.search(v1, 3);

      expect(results.length).toBe(3);
      expect(results[0].id).toBe('v1'); // Most similar to itself
      expect(results[0].similarity).toBeGreaterThan(0.9);
      expect(results[0].metadata?.label).toBe('first');
    });

    it('should remove vectors', () => {
      const store = createScalar8BitStore(dimension);

      store.insert('v1', generateRandomVector());
      store.insert('v2', generateRandomVector());

      expect(store.remove('v1')).toBe(true);
      expect(store.remove('v1')).toBe(false); // Already removed

      const stats = store.getStats();
      expect(stats.count).toBe(1);
    });

    it('should retrieve dequantized vectors', () => {
      const store = createScalar8BitStore(dimension);
      const original = generateRandomVector();

      store.insert('v1', original);

      const retrieved = store.getVector('v1');
      expect(retrieved).not.toBeNull();
      expect(retrieved!.length).toBe(dimension);

      // Check reconstruction error is acceptable
      const error = calculateQuantizationError(original, retrieved!);
      expect(error.meanError).toBeLessThan(0.05);
    });

    it('should export and import store', () => {
      const store = createScalar8BitStore(dimension);

      store.insert('v1', generateRandomVector(), { tag: 'a' });
      store.insert('v2', generateRandomVector(), { tag: 'b' });

      const exported = store.export();

      const store2 = createScalar8BitStore(dimension);
      store2.import(exported);

      const stats = store2.getStats();
      expect(stats.count).toBe(2);

      const v1 = store2.getVector('v1');
      expect(v1).not.toBeNull();
    });
  });

  describe('Scalar 4-bit store', () => {
    it('should achieve higher compression than 8-bit', () => {
      const store4bit = createScalar4BitStore(dimension);
      const store8bit = createScalar8BitStore(dimension);

      for (let i = 0; i < 100; i++) {
        const vec = generateRandomVector();
        store4bit.insert(`v${i}`, vec);
        store8bit.insert(`v${i}`, vec);
      }

      const stats4bit = store4bit.getStats();
      const stats8bit = store8bit.getStats();

      expect(stats4bit.compressionRatio).toBeGreaterThan(stats8bit.compressionRatio);
    });

    it('should still find similar vectors', () => {
      const store = createScalar4BitStore(dimension);

      const baseVector = normalizeVector(generateRandomVector());
      const similarVector = new Float32Array(baseVector);
      // Add small noise
      for (let i = 0; i < dimension; i++) {
        similarVector[i] += (Math.random() - 0.5) * 0.1;
      }

      store.insert('base', baseVector);
      store.insert('similar', normalizeVector(similarVector));
      store.insert('random', normalizeVector(generateRandomVector()));

      const results = store.search(baseVector, 2);

      expect(results.length).toBe(2);
      expect(results[0].id).toBe('base');
      // Similar should be second
      expect(results[1].id).toBe('similar');
    });
  });

  describe('Product Quantized store', () => {
    it('should require training before insertion', async () => {
      const store = createProductQuantizedStore(dimension, 8, 16);

      expect(store.isReady()).toBe(false);

      expect(() => {
        store.insert('v1', generateRandomVector());
      }).toThrow();
    });

    it('should work after training', async () => {
      const store = createProductQuantizedStore(dimension, 8, 16);

      // Generate training data
      const trainingData: Float32Array[] = [];
      for (let i = 0; i < 50; i++) {
        trainingData.push(normalizeVector(generateRandomVector()));
      }

      await store.train(trainingData);
      expect(store.isReady()).toBe(true);

      // Insert vectors
      store.insert('v1', trainingData[0], { index: 0 });
      store.insert('v2', trainingData[1], { index: 1 });

      // Search
      const results = store.search(trainingData[0], 2);
      expect(results.length).toBe(2);
      expect(results[0].id).toBe('v1');
    });

    it('should achieve high compression', async () => {
      const store = createProductQuantizedStore(dimension, 8, 16);

      const trainingData: Float32Array[] = [];
      for (let i = 0; i < 50; i++) {
        trainingData.push(generateRandomVector());
      }

      await store.train(trainingData);

      for (let i = 0; i < 50; i++) {
        store.insert(`v${i}`, trainingData[i]);
      }

      const stats = store.getStats();
      expect(stats.compressionRatio).toBeGreaterThan(4);
    });
  });

  describe('Batch operations', () => {
    it('should insert multiple vectors at once', () => {
      const store = createScalar8BitStore(dimension);

      const items = Array.from({ length: 100 }, (_, i) => ({
        id: `v${i}`,
        vector: generateRandomVector(),
        metadata: { index: i },
      }));

      store.insertBatch(items);

      const stats = store.getStats();
      expect(stats.count).toBe(100);
    });
  });

  describe('Stats and reporting', () => {
    it('should report accurate memory usage', () => {
      const store = createScalar8BitStore(dimension);

      for (let i = 0; i < 100; i++) {
        store.insert(`v${i}`, generateRandomVector());
      }

      const stats = store.getStats();

      expect(stats.count).toBe(100);
      expect(stats.dimension).toBe(dimension);
      expect(stats.quantizationType).toBe('scalar8bit');
      expect(stats.metric).toBe('cosine');
      expect(stats.memoryUsageBytes).toBeGreaterThan(0);
      expect(stats.memoryUsageBytes).toBeLessThan(dimension * 4 * 100); // Less than uncompressed
    });
  });
});
