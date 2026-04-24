/**
 * WASM Vector Operations Benchmark
 *
 * Measures actual performance improvements of WASM-accelerated vector operations
 * compared to pure JavaScript implementations.
 *
 * Run with: npx tsx src/benchmarks/wasm-vector-benchmark.ts
 */

import { WASMVectorSearch } from '../controllers/WASMVectorSearch.js';

interface BenchmarkResult {
  name: string;
  operations: number;
  duration: number;
  opsPerSecond: number;
  avgLatency: number;
}

class VectorBenchmark {
  private dimensions: number;
  private vectorCount: number;

  constructor(dimensions: number = 384, vectorCount: number = 1000) {
    this.dimensions = dimensions;
    this.vectorCount = vectorCount;
  }

  /**
   * Generate random normalized vector
   */
  private generateRandomVector(): Float32Array {
    const vector = new Float32Array(this.dimensions);
    let norm = 0;

    for (let i = 0; i < this.dimensions; i++) {
      vector[i] = Math.random() * 2 - 1;
      norm += vector[i] * vector[i];
    }

    norm = Math.sqrt(norm);
    for (let i = 0; i < this.dimensions; i++) {
      vector[i] /= norm;
    }

    return vector;
  }

  /**
   * Pure JavaScript cosine similarity (baseline)
   */
  private cosineSimilarityJS(a: Float32Array, b: Float32Array): number {
    let dotProduct = 0;
    let normA = 0;
    let normB = 0;

    for (let i = 0; i < a.length; i++) {
      dotProduct += a[i] * b[i];
      normA += a[i] * a[i];
      normB += b[i] * b[i];
    }

    const denom = Math.sqrt(normA) * Math.sqrt(normB);
    return denom === 0 ? 0 : dotProduct / denom;
  }

  /**
   * Benchmark cosine similarity (pure JS)
   */
  async benchmarkJSSimilarity(iterations: number = 10000): Promise<BenchmarkResult> {
    const vectorA = this.generateRandomVector();
    const vectorB = this.generateRandomVector();

    const start = performance.now();

    for (let i = 0; i < iterations; i++) {
      this.cosineSimilarityJS(vectorA, vectorB);
    }

    const duration = performance.now() - start;

    return {
      name: 'Cosine Similarity (Pure JS)',
      operations: iterations,
      duration,
      opsPerSecond: (iterations / duration) * 1000,
      avgLatency: duration / iterations,
    };
  }

  /**
   * Benchmark cosine similarity (WASM-accelerated)
   */
  async benchmarkWASMSimilarity(iterations: number = 10000): Promise<BenchmarkResult> {
    const mockDb = {
      prepare: () => ({ all: () => [], get: () => null, run: () => ({}) }),
      exec: () => {},
    };

    const wasmSearch = new WASMVectorSearch(mockDb, { enableWASM: true });
    const vectorA = this.generateRandomVector();
    const vectorB = this.generateRandomVector();

    const start = performance.now();

    for (let i = 0; i < iterations; i++) {
      wasmSearch.cosineSimilarity(vectorA, vectorB);
    }

    const duration = performance.now() - start;

    return {
      name: 'Cosine Similarity (WASM-Optimized)',
      operations: iterations,
      duration,
      opsPerSecond: (iterations / duration) * 1000,
      avgLatency: duration / iterations,
    };
  }

  /**
   * Benchmark batch similarity operations
   */
  async benchmarkBatchSimilarity(): Promise<BenchmarkResult> {
    const mockDb = {
      prepare: () => ({ all: () => [], get: () => null, run: () => ({}) }),
      exec: () => {},
    };

    const wasmSearch = new WASMVectorSearch(mockDb, { enableWASM: true });
    const query = this.generateRandomVector();
    const vectors: Float32Array[] = [];

    for (let i = 0; i < this.vectorCount; i++) {
      vectors.push(this.generateRandomVector());
    }

    const start = performance.now();
    wasmSearch.batchSimilarity(query, vectors);
    const duration = performance.now() - start;

    return {
      name: 'Batch Similarity Search',
      operations: this.vectorCount,
      duration,
      opsPerSecond: (this.vectorCount / duration) * 1000,
      avgLatency: duration / this.vectorCount,
    };
  }

  /**
   * Benchmark k-NN search
   */
  async benchmarkKNNSearch(k: number = 10): Promise<BenchmarkResult> {
    const mockDb = {
      prepare: () => ({ all: () => [], get: () => null, run: () => ({}) }),
      exec: () => {},
    };

    const wasmSearch = new WASMVectorSearch(mockDb, { enableWASM: true });
    const query = this.generateRandomVector();
    const vectors: Float32Array[] = [];
    const ids: number[] = [];

    for (let i = 0; i < this.vectorCount; i++) {
      vectors.push(this.generateRandomVector());
      ids.push(i);
    }

    // Build index
    wasmSearch.buildIndex(vectors, ids);

    const start = performance.now();
    wasmSearch.searchIndex(query, k);
    const duration = performance.now() - start;

    return {
      name: `k-NN Search (k=${k}, n=${this.vectorCount})`,
      operations: 1,
      duration,
      opsPerSecond: (1 / duration) * 1000,
      avgLatency: duration,
    };
  }

  /**
   * Run all benchmarks
   */
  async runAll(): Promise<void> {
    console.log('='.repeat(80));
    console.log('WASM Vector Operations Benchmark');
    console.log('='.repeat(80));
    console.log(`Configuration: ${this.dimensions}D vectors, ${this.vectorCount} dataset size\n`);

    const results: BenchmarkResult[] = [];

    // Cosine similarity benchmarks
    console.log('Running similarity benchmarks...');
    const jsResult = await this.benchmarkJSSimilarity();
    results.push(jsResult);

    const wasmResult = await this.benchmarkWASMSimilarity();
    results.push(wasmResult);

    const speedup = wasmResult.opsPerSecond / jsResult.opsPerSecond;
    console.log(`  Speedup: ${speedup.toFixed(2)}x\n`);

    // Batch similarity
    console.log('Running batch similarity benchmark...');
    const batchResult = await this.benchmarkBatchSimilarity();
    results.push(batchResult);

    // k-NN search
    console.log('Running k-NN search benchmark...');
    const knnResult = await this.benchmarkKNNSearch();
    results.push(knnResult);

    // Print results table
    console.log('\n' + '='.repeat(80));
    console.log('Results:');
    console.log('='.repeat(80));
    console.log(
      'Benchmark'.padEnd(40) +
      'Operations'.padStart(12) +
      'Duration (ms)'.padStart(15) +
      'Ops/sec'.padStart(15)
    );
    console.log('-'.repeat(80));

    results.forEach(result => {
      console.log(
        result.name.padEnd(40) +
        result.operations.toLocaleString().padStart(12) +
        result.duration.toFixed(2).padStart(15) +
        result.opsPerSecond.toLocaleString(undefined, { maximumFractionDigits: 0 }).padStart(15)
      );
    });

    console.log('='.repeat(80));
    console.log('\nKey Findings:');
    console.log(`- WASM provides ${speedup.toFixed(2)}x speedup for cosine similarity`);
    console.log(`- Batch processing: ${batchResult.opsPerSecond.toFixed(0)} vectors/sec`);
    console.log(`- k-NN search latency: ${knnResult.avgLatency.toFixed(2)}ms`);
    console.log('='.repeat(80));
  }
}

// Run benchmarks
const benchmark = new VectorBenchmark(384, 1000);
benchmark.runAll().catch(console.error);
