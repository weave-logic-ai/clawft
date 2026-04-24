/**
 * Browser WASM Attention Wrapper
 *
 * Provides browser-compatible attention mechanisms with:
 * - Lazy WASM loading
 * - Memory management for WASM linear memory
 * - Fallback to JavaScript when WASM unavailable
 * - Loading states and error handling
 *
 * @module browser/AttentionBrowser
 */
export interface AttentionConfig {
    dimension?: number;
    numHeads?: number;
    blockSize?: number;
    curvature?: number;
    useWASM?: boolean;
}
export interface ConsolidationConfig {
    threshold?: number;
    maxClusters?: number;
    minClusterSize?: number;
}
export type LoadingState = 'idle' | 'loading' | 'loaded' | 'error';
/**
 * Browser-compatible attention class with WASM support
 */
export declare class AttentionBrowser {
    private wasmModule;
    private loadingState;
    private loadError;
    private config;
    constructor(config?: AttentionConfig);
    /**
     * Get current loading state
     */
    getLoadingState(): LoadingState;
    /**
     * Get loading error if any
     */
    getError(): Error | null;
    /**
     * Initialize WASM module (lazy loaded)
     */
    initialize(): Promise<void>;
    /**
     * Flash Attention - Optimized attention mechanism
     * O(N) memory complexity instead of O(N²)
     *
     * @param query - Query vectors
     * @param keys - Key vectors
     * @param values - Value vectors
     * @returns Attention output
     */
    flashAttention(query: Float32Array, keys: Float32Array, values: Float32Array): Promise<Float32Array>;
    /**
     * Hyperbolic Attention - Attention in hyperbolic space
     * Better for hierarchical relationships
     *
     * @param query - Query vector
     * @param keys - Key vectors
     * @returns Similarity scores in hyperbolic space
     */
    hyperbolicAttention(query: Float32Array, keys: Float32Array): Promise<Float32Array>;
    /**
     * Memory Consolidation - Cluster and consolidate similar memories
     *
     * @param memories - Array of memory vectors
     * @param config - Consolidation configuration
     * @returns Consolidated memory clusters
     */
    consolidateMemories(memories: Float32Array[], config?: ConsolidationConfig): Promise<Array<{
        memory: Float32Array;
        count: number;
        members: Float32Array[];
    }>>;
    /**
     * Clean up WASM memory
     */
    dispose(): void;
    private flashAttentionFallback;
    private hyperbolicAttentionFallback;
    private consolidateMemoriesFallback;
    private cosineSimilarity;
}
/**
 * Create attention instance with default config
 */
export declare function createAttention(config?: AttentionConfig): AttentionBrowser;
/**
 * Create attention instance optimized for speed
 */
export declare function createFastAttention(): AttentionBrowser;
/**
 * Create attention instance optimized for quality
 */
export declare function createAccurateAttention(): AttentionBrowser;
//# sourceMappingURL=AttentionBrowser.d.ts.map