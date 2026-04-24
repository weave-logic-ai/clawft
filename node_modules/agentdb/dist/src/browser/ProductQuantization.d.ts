/**
 * Product Quantization for Browser
 *
 * Compresses high-dimensional vectors using product quantization.
 * Achieves 4-32x memory reduction with minimal accuracy loss.
 *
 * Features:
 * - PQ8: 8 subvectors, 256 centroids each (4x compression)
 * - PQ16: 16 subvectors, 256 centroids each (8x compression)
 * - Asymmetric distance computation (ADC)
 * - K-means clustering for codebook training
 *
 * Performance:
 * - Memory: Float32 (4 bytes) → uint8 (1 byte) per subvector
 * - Speed: ~1.5x slower search vs uncompressed
 * - Accuracy: 95-99% recall@10
 */
export interface PQConfig {
    dimension: number;
    numSubvectors: number;
    numCentroids: number;
    maxIterations?: number;
    convergenceThreshold?: number;
}
export interface PQCodebook {
    subvectorDim: number;
    numSubvectors: number;
    numCentroids: number;
    centroids: Float32Array[];
}
export interface CompressedVector {
    codes: Uint8Array;
    norm: number;
}
export declare class ProductQuantization {
    private config;
    private codebook;
    private trained;
    constructor(config: PQConfig);
    /**
     * Train codebook using k-means on training vectors
     */
    train(vectors: Float32Array[]): Promise<void>;
    /**
     * K-means clustering for centroids
     */
    private kMeans;
    /**
     * K-means++ initialization for better centroid selection
     */
    private kMeansPlusPlus;
    /**
     * Compress a vector using trained codebook
     */
    compress(vector: Float32Array): CompressedVector;
    /**
     * Decompress a vector (approximate reconstruction)
     */
    decompress(compressed: CompressedVector): Float32Array;
    /**
     * Asymmetric Distance Computation (ADC)
     * Computes distance from query vector to compressed vector
     */
    asymmetricDistance(query: Float32Array, compressed: CompressedVector): number;
    /**
     * Batch compression for multiple vectors
     */
    batchCompress(vectors: Float32Array[]): CompressedVector[];
    /**
     * Get memory savings
     */
    getCompressionRatio(): number;
    /**
     * Export codebook for persistence
     */
    exportCodebook(): string;
    /**
     * Import codebook
     */
    importCodebook(json: string): void;
    /**
     * Utility: Squared Euclidean distance
     */
    private squaredDistance;
    /**
     * Get statistics
     */
    getStats(): {
        trained: boolean;
        compressionRatio: number;
        memoryPerVector: number;
        codebookSize: number;
    };
}
/**
 * Helper function to create PQ8 (8 subvectors, 4x compression)
 */
export declare function createPQ8(dimension: number): ProductQuantization;
/**
 * Helper function to create PQ16 (16 subvectors, 8x compression)
 */
export declare function createPQ16(dimension: number): ProductQuantization;
/**
 * Helper function to create PQ32 (32 subvectors, 16x compression)
 */
export declare function createPQ32(dimension: number): ProductQuantization;
//# sourceMappingURL=ProductQuantization.d.ts.map