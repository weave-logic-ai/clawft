/**
 * GNN Compatibility Wrapper
 *
 * Fixes API issues with @ruvector/gnn by:
 * 1. Auto-converting regular arrays to Float32Array
 * 2. Providing fallback implementations for broken functions
 * 3. Type-safe interface matching documentation
 */
declare const gnn: any;
export interface SearchResult {
    indices: number[];
    weights: number[];
}
export interface CompressionConfig {
    levelType: 'none' | 'half' | 'pq8' | 'pq4' | 'binary';
    scale?: number;
    subvectors?: number;
    centroids?: number;
    outlierThreshold?: number;
    threshold?: number;
}
/**
 * Fixed differentiableSearch that accepts regular arrays
 * Automatically converts to Float32Array internally
 */
export declare function differentiableSearch(query: number[], candidateEmbeddings: number[][], k: number, temperature?: number): SearchResult;
/**
 * Fallback hierarchicalForward using simple matrix multiplication
 * Since the native implementation is broken
 */
export declare function hierarchicalForward(input: number[], weights: number[] | number[][], inputDim: number, outputDim: number): number[];
/**
 * RuvectorLayer wrapper with fallback
 */
export declare class RuvectorLayer {
    private inputDim;
    private outputDim;
    private weights;
    private activation;
    constructor(inputDim: number, outputDim: number, activation?: 'relu' | 'tanh' | 'sigmoid' | 'none');
    forward(input: number[]): number[];
    private applyActivation;
    getWeights(): number[][];
    setWeights(weights: number[][]): void;
}
/**
 * TensorCompress wrapper with working compression levels
 */
export declare class TensorCompress {
    private config;
    constructor(config: string | CompressionConfig);
    compress(tensor: number[]): number[];
    decompress(compressed: number[]): number[];
    getCompressionRatio(): number;
}
/**
 * Get compression level configuration
 * Fixed version that returns proper config objects
 */
export declare function getCompressionLevel(level: string): CompressionConfig;
/**
 * Check if GNN native module is available and working
 */
export declare function isGNNAvailable(): boolean;
/**
 * Initialize GNN module (if needed)
 */
export declare function initGNN(): void;
export { gnn as gnnRaw };
export type { SearchResult as GNNSearchResult };
//# sourceMappingURL=gnn-wrapper.d.ts.map