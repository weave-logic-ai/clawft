/**
 * AgentDB Browser Advanced Features
 *
 * Unified export for all browser-compatible advanced features.
 *
 * Features:
 * - Product Quantization (PQ8/PQ16/PQ32) - 4-32x memory compression
 * - HNSW Indexing - 10-20x faster approximate search
 * - Graph Neural Networks - Graph attention and message passing
 * - MMR Diversity - Maximal marginal relevance ranking
 * - Tensor Compression - SVD dimension reduction
 * - Batch Operations - Optimized vector processing
 * - WASM Attention - High-performance attention mechanisms (lazy loaded)
 *
 * Bundle Size: ~35 KB minified (~12 KB gzipped)
 * WASM Module: ~157 KB (lazy loaded on demand)
 */

// ============================================================================
// Product Quantization
// ============================================================================

export {
  ProductQuantization,
  createPQ8,
  createPQ16,
  createPQ32,
  type PQConfig,
  type PQCodebook,
  type CompressedVector
} from './ProductQuantization';

// ============================================================================
// HNSW Indexing
// ============================================================================

export {
  HNSWIndex,
  createHNSW,
  createFastHNSW,
  createAccurateHNSW,
  type HNSWConfig,
  type HNSWNode,
  type SearchResult
} from './HNSWIndex';

// ============================================================================
// Advanced Features
// ============================================================================

export {
  GraphNeuralNetwork,
  MaximalMarginalRelevance,
  TensorCompression,
  BatchProcessor,
  type GNNNode,
  type GNNEdge,
  type GNNConfig,
  type MMRConfig
} from './AdvancedFeatures';

// ============================================================================
// WASM Attention (Browser-Compatible)
// ============================================================================

export {
  AttentionBrowser,
  createAttention,
  createFastAttention,
  createAccurateAttention,
  type AttentionConfig,
  type ConsolidationConfig,
  type LoadingState
} from './AttentionBrowser';

// ============================================================================
// Feature Detection
// ============================================================================

/**
 * Detect available browser features
 */
export function detectFeatures() {
  return {
    indexedDB: 'indexedDB' in globalThis,
    broadcastChannel: 'BroadcastChannel' in globalThis,
    webWorkers: typeof (globalThis as any).Worker !== 'undefined',
    wasmSIMD: detectWasmSIMD(),
    sharedArrayBuffer: typeof SharedArrayBuffer !== 'undefined'
  };
}

/**
 * Detect WASM SIMD support
 */
async function detectWasmSIMD(): Promise<boolean> {
  try {
    // Check if WebAssembly is available (browser context)
    if (typeof (globalThis as any).WebAssembly === 'undefined') {
      return false;
    }

    // WASM SIMD detection via feature test
    const simdTest = new Uint8Array([
      0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
      0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7b, 0x03,
      0x02, 0x01, 0x00, 0x0a, 0x0a, 0x01, 0x08, 0x00,
      0xfd, 0x0c, 0xfd, 0x0c, 0xfd, 0x54, 0x0b
    ]);

    const WA = (globalThis as any).WebAssembly;
    const module = await WA.instantiate(simdTest);
    return module instanceof WA.Instance;
  } catch {
    return false;
  }
}

// ============================================================================
// Configuration Presets
// ============================================================================

/**
 * Recommended configuration for small datasets (<1K vectors)
 */
export const SMALL_DATASET_CONFIG = {
  pq: { enabled: false },
  hnsw: { enabled: false },
  gnn: { enabled: true, numHeads: 2 },
  mmr: { enabled: true, lambda: 0.7 },
  svd: { enabled: false }
};

/**
 * Recommended configuration for medium datasets (1K-10K vectors)
 */
export const MEDIUM_DATASET_CONFIG = {
  pq: { enabled: true, subvectors: 8 },
  hnsw: { enabled: true, M: 16 },
  gnn: { enabled: true, numHeads: 4 },
  mmr: { enabled: true, lambda: 0.7 },
  svd: { enabled: false }
};

/**
 * Recommended configuration for large datasets (10K-100K vectors)
 */
export const LARGE_DATASET_CONFIG = {
  pq: { enabled: true, subvectors: 16 },
  hnsw: { enabled: true, M: 32 },
  gnn: { enabled: true, numHeads: 4 },
  mmr: { enabled: true, lambda: 0.7 },
  svd: { enabled: true, targetDim: 128 }
};

/**
 * Memory-optimized configuration (minimal memory usage)
 */
export const MEMORY_OPTIMIZED_CONFIG = {
  pq: { enabled: true, subvectors: 32 },  // 16x compression
  hnsw: { enabled: true, M: 8 },  // Fewer connections
  gnn: { enabled: false },
  mmr: { enabled: false },
  svd: { enabled: true, targetDim: 64 }  // Aggressive dimension reduction
};

/**
 * Speed-optimized configuration (fastest search)
 */
export const SPEED_OPTIMIZED_CONFIG = {
  pq: { enabled: false },  // No compression overhead
  hnsw: { enabled: true, M: 32, efSearch: 100 },  // Maximum HNSW quality
  gnn: { enabled: false },
  mmr: { enabled: false },
  svd: { enabled: false }
};

/**
 * Quality-optimized configuration (best result quality)
 */
export const QUALITY_OPTIMIZED_CONFIG = {
  pq: { enabled: false },  // No compression
  hnsw: { enabled: true, M: 48, efConstruction: 400 },  // Highest quality
  gnn: { enabled: true, numHeads: 8 },  // More attention heads
  mmr: { enabled: true, lambda: 0.8 },  // More diversity
  svd: { enabled: false }  // No dimension loss
};

// ============================================================================
// Version Information
// ============================================================================

export const VERSION = {
  major: 2,
  minor: 0,
  patch: 0,
  prerelease: 'alpha.2',
  features: 'advanced',
  full: '2.0.0-alpha.2+advanced'
};

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Estimate memory usage for configuration
 */
export function estimateMemoryUsage(
  numVectors: number,
  dimension: number,
  config: any
): {
  vectors: number;
  index: number;
  total: number;
  totalMB: number;
} {
  let vectorBytes = numVectors * dimension * 4;  // Float32Array

  // Apply PQ compression
  if (config.pq?.enabled) {
    const subvectors = config.pq.subvectors || 8;
    vectorBytes = numVectors * (subvectors + 4);  // codes + norm
  }

  // Apply SVD compression
  if (config.svd?.enabled) {
    const targetDim = config.svd.targetDim || dimension / 2;
    vectorBytes = numVectors * targetDim * 4;
  }

  // HNSW index overhead
  let indexBytes = 0;
  if (config.hnsw?.enabled) {
    const M = config.hnsw.M || 16;
    const avgConnections = M * 1.5;  // Estimate
    indexBytes = numVectors * avgConnections * 4;  // Connection IDs
  }

  const total = vectorBytes + indexBytes;

  return {
    vectors: vectorBytes,
    index: indexBytes,
    total,
    totalMB: total / (1024 * 1024)
  };
}

/**
 * Recommend configuration based on dataset size
 */
export function recommendConfig(numVectors: number, dimension: number) {
  if (numVectors < 1000) {
    return {
      name: 'SMALL_DATASET',
      config: SMALL_DATASET_CONFIG,
      reason: 'Small dataset, linear search is fast enough'
    };
  } else if (numVectors < 10000) {
    return {
      name: 'MEDIUM_DATASET',
      config: MEDIUM_DATASET_CONFIG,
      reason: 'Medium dataset, HNSW + PQ8 recommended'
    };
  } else {
    return {
      name: 'LARGE_DATASET',
      config: LARGE_DATASET_CONFIG,
      reason: 'Large dataset, aggressive compression + HNSW recommended'
    };
  }
}

/**
 * Benchmark search performance
 */
export async function benchmarkSearch(
  searchFn: (query: Float32Array, k: number) => any[],
  numQueries: number = 100,
  k: number = 10,
  dimension: number = 384
): Promise<{
  avgTimeMs: number;
  minTimeMs: number;
  maxTimeMs: number;
  p50Ms: number;
  p95Ms: number;
  p99Ms: number;
}> {
  const times: number[] = [];

  for (let i = 0; i < numQueries; i++) {
    const query = new Float32Array(dimension);
    for (let d = 0; d < dimension; d++) {
      query[d] = Math.random() - 0.5;
    }

    const start = performance.now();
    searchFn(query, k);
    const end = performance.now();

    times.push(end - start);
  }

  times.sort((a, b) => a - b);

  return {
    avgTimeMs: times.reduce((a, b) => a + b, 0) / times.length,
    minTimeMs: times[0],
    maxTimeMs: times[times.length - 1],
    p50Ms: times[Math.floor(times.length * 0.5)],
    p95Ms: times[Math.floor(times.length * 0.95)],
    p99Ms: times[Math.floor(times.length * 0.99)]
  };
}
