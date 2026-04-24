/**
 * Vector Quantization for AgentDB
 *
 * Provides memory-efficient vector storage through quantization techniques:
 *
 * 1. Scalar Quantization (8-bit and 4-bit)
 *    - 8-bit: 4x memory reduction (Float32 -> Uint8)
 *    - 4-bit: 8x memory reduction (Float32 -> 4-bit packed)
 *
 * 2. Product Quantization
 *    - Configurable subspaces and centroids
 *    - Higher compression ratios (up to 32x+)
 *    - Requires training on representative data
 *
 * 3. QuantizedVectorStore
 *    - Wraps vector collections with quantized storage
 *    - Asymmetric distance computation (full precision queries)
 *    - On-the-fly dequantization for similarity calculations
 */
/** @inline Maximum allowed vector dimension to prevent DoS via large allocations */
export declare const MAX_VECTOR_DIMENSION = 4096;
/** @inline Maximum batch size for optimal cache utilization */
export declare const MAX_BATCH_SIZE = 10000;
/** @inline Default cache size for embedding storage */
export declare const DEFAULT_CACHE_SIZE = 10000;
/**
 * Statistics for quantization operations
 */
export interface QuantizationStats {
    /** Minimum value in the original vector */
    min: number;
    /** Maximum value in the original vector */
    max: number;
    /** Mean quantization error (per element) */
    meanError?: number;
    /** Maximum quantization error (per element) */
    maxError?: number;
    /** Compression ratio achieved */
    compressionRatio: number;
}
/**
 * Quantized vector with metadata for dequantization
 */
export interface QuantizedVector {
    /** The quantized data */
    data: Uint8Array;
    /** Minimum value used for quantization */
    min: number;
    /** Maximum value used for quantization */
    max: number;
    /** Original vector dimension */
    dimension: number;
    /** Quantization type */
    type: '8bit' | '4bit';
}
/**
 * Configuration for Product Quantizer
 */
export interface ProductQuantizerConfig {
    /** Vector dimension (must be divisible by numSubspaces) */
    dimension: number;
    /** Number of subspaces to divide the vector into */
    numSubspaces: number;
    /** Number of centroids per subspace (max 256 for uint8 codes) */
    numCentroids: number;
    /** Maximum iterations for k-means training */
    maxIterations?: number;
    /** Convergence threshold for k-means */
    convergenceThreshold?: number;
    /** Seed for reproducible random initialization */
    seed?: number;
}
/**
 * Encoded vector from Product Quantization
 */
export interface PQEncodedVector {
    /** Centroid indices for each subspace */
    codes: Uint8Array;
    /** Original vector norm (for normalized distance computation) */
    norm: number;
}
/**
 * Configuration for QuantizedVectorStore
 */
export interface QuantizedVectorStoreConfig {
    /** Vector dimension */
    dimension: number;
    /** Quantization method to use */
    quantizationType: 'scalar8bit' | 'scalar4bit' | 'product';
    /** Product quantization config (required if quantizationType is 'product') */
    productQuantizerConfig?: Omit<ProductQuantizerConfig, 'dimension'>;
    /** Distance metric for similarity calculations */
    metric?: 'cosine' | 'l2' | 'ip';
}
/**
 * Search result from QuantizedVectorStore
 */
export interface QuantizedSearchResult {
    /** Vector ID */
    id: string;
    /** Distance (lower is more similar for L2, higher for cosine/IP) */
    distance: number;
    /** Normalized similarity score (0-1, higher is more similar) */
    similarity: number;
    /** Optional metadata */
    metadata?: Record<string, unknown>;
}
/**
 * Quantize a vector to 8-bit values (4x memory reduction)
 *
 * Formula: quantized = round((value - min) / (max - min) * 255)
 *
 * @param vector - Input vector as Float32Array
 * @returns Object containing quantized data and min/max for dequantization
 */
export declare function quantize8bit(vector: Float32Array): QuantizedVector;
/**
 * Quantize a vector to 4-bit values (8x memory reduction)
 *
 * Formula: quantized = round((value - min) / (max - min) * 15)
 * Two 4-bit values are packed into each byte.
 *
 * @param vector - Input vector as Float32Array
 * @returns Object containing quantized data (packed) and min/max for dequantization
 */
export declare function quantize4bit(vector: Float32Array): QuantizedVector;
/**
 * Dequantize an 8-bit vector back to Float32
 *
 * @param quantized - Quantized data as Uint8Array
 * @param min - Minimum value from original quantization
 * @param max - Maximum value from original quantization
 * @returns Reconstructed Float32Array
 */
export declare function dequantize8bit(quantized: Uint8Array, min: number, max: number): Float32Array;
/**
 * Dequantize a 4-bit packed vector back to Float32
 *
 * @param quantized - Packed quantized data as Uint8Array
 * @param min - Minimum value from original quantization
 * @param max - Maximum value from original quantization
 * @param originalLength - Original vector length (needed due to packing)
 * @returns Reconstructed Float32Array
 */
export declare function dequantize4bit(quantized: Uint8Array, min: number, max: number, originalLength?: number): Float32Array;
/**
 * Calculate quantization error statistics
 *
 * @param original - Original vector
 * @param reconstructed - Reconstructed vector after quantization/dequantization
 * @returns Statistics about the quantization error
 */
export declare function calculateQuantizationError(original: Float32Array, reconstructed: Float32Array): {
    meanError: number;
    maxError: number;
    mse: number;
};
/**
 * Get quantization statistics for a vector
 *
 * @param vector - Original vector
 * @param type - Quantization type ('8bit' or '4bit')
 * @returns Quantization statistics including error metrics
 */
export declare function getQuantizationStats(vector: Float32Array, type: '8bit' | '4bit'): QuantizationStats;
/**
 * Product Quantizer for high compression ratios
 *
 * Divides vectors into subspaces and learns centroids for each subspace.
 * Vectors are encoded as indices into the centroid codebooks.
 */
export declare class ProductQuantizer {
    private config;
    private subspaceDim;
    private codebooks;
    private trained;
    private rng;
    constructor(config: ProductQuantizerConfig);
    /**
     * Train the codebooks using k-means clustering
     *
     * @param vectors - Training vectors (should be representative of the data)
     */
    train(vectors: Float32Array[]): Promise<void>;
    /**
     * K-means clustering with k-means++ initialization
     */
    private kMeans;
    /**
     * K-means++ initialization for better centroid selection
     */
    private kMeansPlusPlus;
    /**
     * Encode a vector using the trained codebooks
     *
     * @param vector - Vector to encode
     * @returns Encoded vector with centroid codes and norm
     */
    encode(vector: Float32Array): PQEncodedVector;
    /**
     * Decode an encoded vector (approximate reconstruction)
     *
     * @param encoded - Encoded vector
     * @returns Reconstructed Float32Array
     */
    decode(encoded: PQEncodedVector): Float32Array;
    /**
     * Compute asymmetric distance between a query and encoded vector
     * Query is in full precision, database vector is encoded
     *
     * @param query - Full precision query vector
     * @param encoded - Encoded database vector
     * @returns Squared L2 distance
     */
    asymmetricDistance(query: Float32Array, encoded: PQEncodedVector): number;
    /**
     * Precompute distance tables for efficient batch search
     * Tables[s][c] = squared distance from query subvector s to centroid c
     *
     * @param query - Query vector
     * @returns Distance lookup tables for each subspace
     */
    precomputeDistanceTables(query: Float32Array): Float32Array[];
    /**
     * Compute distance using precomputed tables (very fast)
     *
     * @param tables - Precomputed distance tables from precomputeDistanceTables()
     * @param encoded - Encoded vector
     * @returns Squared L2 distance
     */
    distanceFromTables(tables: Float32Array[], encoded: PQEncodedVector): number;
    /**
     * Get compression ratio
     */
    getCompressionRatio(): number;
    /**
     * Get quantizer statistics
     */
    getStats(): {
        trained: boolean;
        dimension: number;
        numSubspaces: number;
        subspaceDim: number;
        numCentroids: number;
        compressionRatio: number;
        codebookSizeBytes: number;
    };
    /**
     * Export codebooks for persistence
     */
    exportCodebooks(): string;
    /**
     * Import codebooks from persisted data
     * Includes validation to prevent prototype pollution and type confusion
     */
    importCodebooks(json: string): void;
    /**
     * Check if the quantizer is trained
     */
    isTrained(): boolean;
    /**
     * Squared L2 distance utility
     */
    private squaredL2Distance;
}
/**
 * QuantizedVectorStore - Memory-efficient vector storage
 *
 * Wraps vector collections with quantized storage while providing
 * full-precision query support through asymmetric distance computation.
 */
export declare class QuantizedVectorStore {
    private config;
    private vectors;
    private productQuantizer;
    constructor(config: QuantizedVectorStoreConfig);
    /**
     * Train the product quantizer (required for 'product' quantization type)
     *
     * @param vectors - Training vectors
     */
    train(vectors: Float32Array[]): Promise<void>;
    /**
     * Check if the store is ready for insertions
     */
    isReady(): boolean;
    /**
     * Insert a vector into the store
     *
     * @param id - Unique identifier
     * @param vector - Vector to store
     * @param metadata - Optional metadata
     */
    insert(id: string, vector: Float32Array, metadata?: Record<string, unknown>): void;
    /**
     * Insert multiple vectors in batch
     *
     * @param items - Array of {id, vector, metadata}
     */
    insertBatch(items: Array<{
        id: string;
        vector: Float32Array;
        metadata?: Record<string, unknown>;
    }>): void;
    /**
     * Search for nearest neighbors
     * Uses asymmetric distance computation: query in full precision,
     * database vectors in quantized form.
     *
     * @param query - Query vector (full precision)
     * @param k - Number of results to return
     * @param threshold - Optional similarity threshold
     * @returns Sorted search results (most similar first)
     */
    search(query: Float32Array, k: number, threshold?: number): QuantizedSearchResult[];
    /**
     * Remove a vector by ID
     *
     * @param id - Vector ID to remove
     * @returns true if removed, false if not found
     */
    remove(id: string): boolean;
    /**
     * Get a dequantized vector by ID
     *
     * @param id - Vector ID
     * @returns Reconstructed vector or null if not found
     */
    getVector(id: string): Float32Array | null;
    /**
     * Get store statistics
     */
    getStats(): {
        count: number;
        dimension: number;
        quantizationType: string;
        metric: string;
        compressionRatio: number;
        memoryUsageBytes: number;
        productQuantizerStats?: ReturnType<ProductQuantizer['getStats']>;
    };
    /**
     * Clear all vectors
     */
    clear(): void;
    /**
     * Export store to JSON (for persistence)
     */
    export(): string;
    /**
     * Import store from JSON
     */
    import(json: string): void;
    /**
     * Compute asymmetric distance (query full precision, database quantized)
     */
    private computeAsymmetricDistance;
    /**
     * Dequantize a stored vector
     */
    private dequantizeStored;
    /**
     * Normalize a vector to unit length
     */
    private normalize;
    /**
     * Squared L2 distance
     */
    private squaredL2Distance;
}
/**
 * Create an 8-bit scalar quantized vector store (4x compression)
 */
export declare function createScalar8BitStore(dimension: number, metric?: 'cosine' | 'l2' | 'ip'): QuantizedVectorStore;
/**
 * Create a 4-bit scalar quantized vector store (8x compression)
 */
export declare function createScalar4BitStore(dimension: number, metric?: 'cosine' | 'l2' | 'ip'): QuantizedVectorStore;
/**
 * Create a product quantized vector store (configurable compression)
 *
 * @param dimension - Vector dimension
 * @param numSubspaces - Number of subspaces (8, 16, 32, 64 typical)
 * @param numCentroids - Number of centroids per subspace (256 max for uint8)
 * @param metric - Distance metric
 */
export declare function createProductQuantizedStore(dimension: number, numSubspaces?: number, numCentroids?: number, metric?: 'cosine' | 'l2' | 'ip'): QuantizedVectorStore;
//# sourceMappingURL=vector-quantization.d.ts.map