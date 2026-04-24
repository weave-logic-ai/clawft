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
export { ProductQuantization, createPQ8, createPQ16, createPQ32, type PQConfig, type PQCodebook, type CompressedVector } from './ProductQuantization';
export { HNSWIndex, createHNSW, createFastHNSW, createAccurateHNSW, type HNSWConfig, type HNSWNode, type SearchResult } from './HNSWIndex';
export { GraphNeuralNetwork, MaximalMarginalRelevance, TensorCompression, BatchProcessor, type GNNNode, type GNNEdge, type GNNConfig, type MMRConfig } from './AdvancedFeatures';
export { AttentionBrowser, createAttention, createFastAttention, createAccurateAttention, type AttentionConfig, type ConsolidationConfig, type LoadingState } from './AttentionBrowser';
/**
 * Detect available browser features
 */
export declare function detectFeatures(): {
    indexedDB: boolean;
    broadcastChannel: boolean;
    webWorkers: boolean;
    wasmSIMD: Promise<boolean>;
    sharedArrayBuffer: boolean;
};
/**
 * Recommended configuration for small datasets (<1K vectors)
 */
export declare const SMALL_DATASET_CONFIG: {
    pq: {
        enabled: boolean;
    };
    hnsw: {
        enabled: boolean;
    };
    gnn: {
        enabled: boolean;
        numHeads: number;
    };
    mmr: {
        enabled: boolean;
        lambda: number;
    };
    svd: {
        enabled: boolean;
    };
};
/**
 * Recommended configuration for medium datasets (1K-10K vectors)
 */
export declare const MEDIUM_DATASET_CONFIG: {
    pq: {
        enabled: boolean;
        subvectors: number;
    };
    hnsw: {
        enabled: boolean;
        M: number;
    };
    gnn: {
        enabled: boolean;
        numHeads: number;
    };
    mmr: {
        enabled: boolean;
        lambda: number;
    };
    svd: {
        enabled: boolean;
    };
};
/**
 * Recommended configuration for large datasets (10K-100K vectors)
 */
export declare const LARGE_DATASET_CONFIG: {
    pq: {
        enabled: boolean;
        subvectors: number;
    };
    hnsw: {
        enabled: boolean;
        M: number;
    };
    gnn: {
        enabled: boolean;
        numHeads: number;
    };
    mmr: {
        enabled: boolean;
        lambda: number;
    };
    svd: {
        enabled: boolean;
        targetDim: number;
    };
};
/**
 * Memory-optimized configuration (minimal memory usage)
 */
export declare const MEMORY_OPTIMIZED_CONFIG: {
    pq: {
        enabled: boolean;
        subvectors: number;
    };
    hnsw: {
        enabled: boolean;
        M: number;
    };
    gnn: {
        enabled: boolean;
    };
    mmr: {
        enabled: boolean;
    };
    svd: {
        enabled: boolean;
        targetDim: number;
    };
};
/**
 * Speed-optimized configuration (fastest search)
 */
export declare const SPEED_OPTIMIZED_CONFIG: {
    pq: {
        enabled: boolean;
    };
    hnsw: {
        enabled: boolean;
        M: number;
        efSearch: number;
    };
    gnn: {
        enabled: boolean;
    };
    mmr: {
        enabled: boolean;
    };
    svd: {
        enabled: boolean;
    };
};
/**
 * Quality-optimized configuration (best result quality)
 */
export declare const QUALITY_OPTIMIZED_CONFIG: {
    pq: {
        enabled: boolean;
    };
    hnsw: {
        enabled: boolean;
        M: number;
        efConstruction: number;
    };
    gnn: {
        enabled: boolean;
        numHeads: number;
    };
    mmr: {
        enabled: boolean;
        lambda: number;
    };
    svd: {
        enabled: boolean;
    };
};
export declare const VERSION: {
    major: number;
    minor: number;
    patch: number;
    prerelease: string;
    features: string;
    full: string;
};
/**
 * Estimate memory usage for configuration
 */
export declare function estimateMemoryUsage(numVectors: number, dimension: number, config: any): {
    vectors: number;
    index: number;
    total: number;
    totalMB: number;
};
/**
 * Recommend configuration based on dataset size
 */
export declare function recommendConfig(numVectors: number, dimension: number): {
    name: string;
    config: {
        pq: {
            enabled: boolean;
        };
        hnsw: {
            enabled: boolean;
        };
        gnn: {
            enabled: boolean;
            numHeads: number;
        };
        mmr: {
            enabled: boolean;
            lambda: number;
        };
        svd: {
            enabled: boolean;
        };
    };
    reason: string;
};
/**
 * Benchmark search performance
 */
export declare function benchmarkSearch(searchFn: (query: Float32Array, k: number) => any[], numQueries?: number, k?: number, dimension?: number): Promise<{
    avgTimeMs: number;
    minTimeMs: number;
    maxTimeMs: number;
    p50Ms: number;
    p95Ms: number;
    p99Ms: number;
}>;
//# sourceMappingURL=index.d.ts.map