/**
 * WASMVectorSearch - High-Performance Vector Operations
 *
 * Accelerates vector similarity search using ReasoningBank WASM module.
 * Provides 10-50x speedup for cosine similarity calculations compared to pure JS.
 *
 * Features:
 * - WASM-accelerated similarity search
 * - Batch vector operations
 * - Approximate nearest neighbors for large datasets
 * - Graceful fallback to JavaScript
 * - SIMD optimizations when available
 */
type Database = any;
export interface VectorSearchConfig {
    enableWASM: boolean;
    enableSIMD: boolean;
    batchSize: number;
    indexThreshold: number;
}
export interface VectorSearchResult {
    id: number;
    distance: number;
    similarity: number;
    metadata?: any;
}
export interface VectorIndex {
    vectors: Float32Array[];
    ids: number[];
    metadata: any[];
    built: boolean;
    lastUpdate: number;
}
export declare class WASMVectorSearch {
    private db;
    private config;
    private wasmModule;
    private wasmAvailable;
    private simdAvailable;
    private vectorIndex;
    private wasmInitPromise;
    constructor(db: Database, config?: Partial<VectorSearchConfig>);
    /**
     * Wait for WASM initialization to complete
     */
    waitForInit(): Promise<boolean>;
    /**
     * Get the directory of the current module
     */
    private getCurrentModuleDir;
    /**
     * Build list of potential WASM module paths
     */
    private getWASMSearchPaths;
    /**
     * Initialize WASM module with robust path resolution
     */
    private initializeWASM;
    /**
     * Detect SIMD support
     */
    private detectSIMD;
    /**
     * Calculate cosine similarity between two vectors (optimized)
     */
    cosineSimilarity(a: Float32Array, b: Float32Array): number;
    /**
     * Batch calculate similarities between query and multiple vectors
     */
    batchSimilarity(query: Float32Array, vectors: Float32Array[]): number[];
    /**
     * Find k-nearest neighbors using brute force search
     */
    findKNN(query: Float32Array, k: number, tableName?: string, options?: {
        threshold?: number;
        filters?: Record<string, any>;
    }): Promise<VectorSearchResult[]>;
    /**
     * Build approximate nearest neighbor index for large datasets
     */
    buildIndex(vectors: Float32Array[], ids: number[], metadata?: any[]): void;
    /**
     * Search using ANN index (if available)
     */
    searchIndex(query: Float32Array, k: number, threshold?: number): VectorSearchResult[];
    /**
     * Get vector search statistics
     */
    getStats(): {
        wasmAvailable: boolean;
        simdAvailable: boolean;
        indexBuilt: boolean;
        indexSize: number;
        lastIndexUpdate: number | null;
    };
    /**
     * Clear vector index
     */
    clearIndex(): void;
}
export {};
//# sourceMappingURL=WASMVectorSearch.d.ts.map