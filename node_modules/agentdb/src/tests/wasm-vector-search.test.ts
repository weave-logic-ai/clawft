/**
 * WASM Vector Search Tests
 *
 * Integration tests for WASM-accelerated vector operations
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { WASMVectorSearch } from '../controllers/WASMVectorSearch.js';
import { EnhancedEmbeddingService } from '../controllers/EnhancedEmbeddingService.js';

describe('WASMVectorSearch', () => {
  let mockDb: any;
  let wasmSearch: WASMVectorSearch;

  beforeEach(() => {
    mockDb = {
      prepare: () => ({ all: () => [], get: () => null, run: () => ({ lastInsertRowid: 1, changes: 1 }) }),
      exec: () => {},
    };

    wasmSearch = new WASMVectorSearch(mockDb);
  });

  describe('Cosine Similarity', () => {
    it('should calculate cosine similarity correctly', () => {
      const vectorA = new Float32Array([1, 0, 0]);
      const vectorB = new Float32Array([1, 0, 0]);

      const similarity = wasmSearch.cosineSimilarity(vectorA, vectorB);
      expect(similarity).toBeCloseTo(1.0, 5);
    });

    it('should handle orthogonal vectors', () => {
      const vectorA = new Float32Array([1, 0, 0]);
      const vectorB = new Float32Array([0, 1, 0]);

      const similarity = wasmSearch.cosineSimilarity(vectorA, vectorB);
      expect(similarity).toBeCloseTo(0.0, 5);
    });

    it('should handle opposite vectors', () => {
      const vectorA = new Float32Array([1, 0, 0]);
      const vectorB = new Float32Array([-1, 0, 0]);

      const similarity = wasmSearch.cosineSimilarity(vectorA, vectorB);
      expect(similarity).toBeCloseTo(-1.0, 5);
    });

    it('should throw error for mismatched dimensions', () => {
      const vectorA = new Float32Array([1, 0, 0]);
      const vectorB = new Float32Array([1, 0]);

      expect(() => wasmSearch.cosineSimilarity(vectorA, vectorB)).toThrow();
    });
  });

  describe('Batch Similarity', () => {
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

    it('should handle large batches efficiently', () => {
      const query = new Float32Array(384).fill(0.5);
      const vectors: Float32Array[] = [];

      for (let i = 0; i < 1000; i++) {
        vectors.push(new Float32Array(384).fill(Math.random()));
      }

      const startTime = performance.now();
      const similarities = wasmSearch.batchSimilarity(query, vectors);
      const duration = performance.now() - startTime;

      expect(similarities).toHaveLength(1000);
      expect(duration).toBeLessThan(1000); // Should complete in under 1 second
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
      expect(results[0].id).toBe(1); // Most similar
      expect(results[0].similarity).toBeCloseTo(1.0, 5);
    });

    it('should clear index', () => {
      const vectors = [new Float32Array([1, 0, 0])];
      const ids = [1];

      wasmSearch = new WASMVectorSearch(mockDb, { indexThreshold: 0 });
      wasmSearch.buildIndex(vectors, ids);

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

  describe('Batch Processing', () => {
    it('should embed batch of texts', async () => {
      const texts = ['hello', 'world', 'test', 'embedding', 'service'];
      const embeddings = await service.embedBatch(texts);

      expect(embeddings).toHaveLength(5);
      embeddings.forEach(emb => {
        expect(emb).toBeInstanceOf(Float32Array);
        expect(emb.length).toBe(384);
      });
    });

    it('should handle large batches efficiently', async () => {
      const texts = Array.from({ length: 200 }, (_, i) => `text ${i}`);

      const startTime = performance.now();
      const embeddings = await service.embedBatch(texts);
      const duration = performance.now() - startTime;

      expect(embeddings).toHaveLength(200);
      console.log(`Batch embedding of 200 texts took ${duration.toFixed(2)}ms`);
    });
  });

  describe('Similarity', () => {
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

      // Note: Mock embeddings are deterministic but not semantic,
      // so we just verify the structure is correct
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
});
