/**
 * Vector Quantization Module for AgentDB
 *
 * Provides memory-efficient vector storage through quantization techniques:
 * - Scalar Quantization (4-bit and 8-bit)
 * - Product Quantization with trainable codebooks
 * - Quantized Vector Store with asymmetric distance computation
 *
 * @module optimizations/Quantization
 */
/**
 * Supported quantization types
 */
export type QuantizationType = 'scalar-4bit' | 'scalar-8bit' | 'product';
/**
 * Configuration for quantization operations
 */
export interface QuantizationConfig {
    /** Type of quantization to apply */
    type: QuantizationType;
    /** Number of subvectors for product quantization (default: 8) */
    numSubvectors?: number;
    /** Number of centroids per subvector for product quantization (default: 256) */
    numCentroids?: number;
    /** Number of training iterations for product quantization (default: 25) */
    trainingIterations?: number;
}
/**
 * Container for quantized vector data
 */
export interface QuantizedVectors {
    /** The quantization type used */
    type: QuantizationType;
    /** Original vector dimension */
    dimension: number;
    /** Number of vectors stored */
    count: number;
    /** Quantized data buffer */
    data: Uint8Array;
    /** Minimum values per dimension (for scalar quantization) */
    mins?: Float32Array;
    /** Maximum values per dimension (for scalar quantization) */
    maxs?: Float32Array;
    /** Codebooks for product quantization */
    codebooks?: Float32Array[];
    /** Number of subvectors (for product quantization) */
    numSubvectors?: number;
    /** Subvector dimension (for product quantization) */
    subvectorDim?: number;
}
/**
 * Search result from quantized vector store
 */
export interface SearchResult {
    /** Unique identifier of the vector */
    id: string;
    /** Distance/similarity score */
    distance: number;
    /** Original index in the store */
    index: number;
}
/**
 * Scalar quantization reduces memory by mapping floating-point values
 * to lower-bit integer representations.
 *
 * @example
 * ```typescript
 * const sq = new ScalarQuantization();
 * const vectors = [new Float32Array([0.1, 0.5, 0.9])];
 * const quantized = sq.quantize(vectors, 8);
 * const restored = sq.dequantize(quantized);
 * ```
 */
export declare class ScalarQuantization {
    /**
     * Quantize vectors to reduced bit representation
     *
     * @param vectors - Array of vectors to quantize
     * @param bits - Target bit depth (4 or 8)
     * @returns Quantized vector container
     */
    quantize(vectors: Float32Array[], bits: 4 | 8): QuantizedVectors;
    /**
     * Dequantize vectors back to floating-point representation
     *
     * @param quantized - Quantized vector container
     * @returns Array of reconstructed vectors
     */
    dequantize(quantized: QuantizedVectors): Float32Array[];
    /**
     * Compute approximate squared Euclidean distance using quantized representation
     *
     * @param quantized - Quantized vectors
     * @param index - Index of vector in quantized store
     * @param query - Full-precision query vector
     * @returns Approximate squared distance
     */
    computeDistance(quantized: QuantizedVectors, index: number, query: Float32Array): number;
}
/**
 * Product quantization divides vectors into subvectors and learns
 * codebooks for each subspace, enabling high compression ratios.
 *
 * @example
 * ```typescript
 * const pq = new ProductQuantization(8, 256);
 * pq.train(trainingVectors);
 * const encoded = pq.encode(vectors);
 * const decoded = pq.decode(encoded);
 * ```
 */
export declare class ProductQuantization {
    /** Number of subvectors to divide each vector into */
    private readonly numSubvectors;
    /** Number of centroids per subvector codebook */
    private readonly numCentroids;
    /** Maximum training iterations */
    private readonly maxIterations;
    /** Codebooks: one per subvector, each containing numCentroids centroids */
    private codebooks;
    /** Dimension of each subvector */
    private subvectorDim;
    /** Total vector dimension */
    private dimension;
    /** Whether the model has been trained */
    private trained;
    /**
     * Create a product quantization instance
     *
     * @param numSubvectors - Number of subvectors (default: 8)
     * @param numCentroids - Centroids per subvector (default: 256, max 256 for byte encoding)
     * @param maxIterations - Maximum k-means iterations (default: 25)
     */
    constructor(numSubvectors?: number, numCentroids?: number, maxIterations?: number);
    /**
     * Train codebooks from sample vectors using k-means clustering
     *
     * @param vectors - Training vectors
     */
    train(vectors: Float32Array[]): void;
    /**
     * Train a single codebook using k-means clustering
     *
     * @param subvectors - Subvectors for this segment
     * @returns Codebook as flattened Float32Array
     */
    private trainCodebook;
    /**
     * Encode vectors using trained codebooks
     *
     * @param vectors - Vectors to encode
     * @returns Quantized vector container
     */
    encode(vectors: Float32Array[]): QuantizedVectors;
    /**
     * Decode quantized vectors back to approximate floating-point representation
     *
     * @param quantized - Quantized vector container
     * @returns Array of reconstructed vectors
     */
    decode(quantized: QuantizedVectors): Float32Array[];
    /**
     * Compute squared Euclidean distance between two vectors
     */
    private squaredDistance;
    /**
     * Check if the model has been trained
     */
    isTrained(): boolean;
    /**
     * Get codebooks for serialization
     */
    getCodebooks(): Float32Array[];
    /**
     * Load pre-trained codebooks
     *
     * @param codebooks - Array of codebook Float32Arrays
     * @param dimension - Original vector dimension
     */
    loadCodebooks(codebooks: Float32Array[], dimension: number): void;
}
/**
 * A vector store that maintains vectors in quantized form for memory efficiency.
 * Uses asymmetric distance computation for improved accuracy during search.
 *
 * @example
 * ```typescript
 * const store = new QuantizedVectorStore({ type: 'scalar-8bit' });
 * store.add('vec1', new Float32Array([0.1, 0.2, 0.3]));
 * const results = store.search(new Float32Array([0.1, 0.2, 0.3]), 5);
 * ```
 */
export declare class QuantizedVectorStore {
    /** Quantization configuration */
    private readonly config;
    /** Scalar quantization instance */
    private scalarQuantizer;
    /** Product quantization instance */
    private productQuantizer;
    /** ID to index mapping */
    private idToIndex;
    /** Index to ID mapping */
    private indexToId;
    /** Raw vectors buffer (before quantization) */
    private rawVectors;
    /** Quantized vectors */
    private quantizedData;
    /** Whether the store needs reindexing */
    private dirty;
    /** Batch size for auto-reindexing */
    private readonly batchSize;
    /**
     * Create a quantized vector store
     *
     * @param config - Quantization configuration
     */
    constructor(config: QuantizationConfig);
    /**
     * Add a vector to the store
     *
     * @param id - Unique identifier for the vector
     * @param vector - The vector to add
     */
    add(id: string, vector: Float32Array): void;
    /**
     * Add multiple vectors in batch
     *
     * @param entries - Array of [id, vector] pairs
     */
    addBatch(entries: Array<[string, Float32Array]>): void;
    /**
     * Remove a vector from the store
     *
     * @param id - ID of vector to remove
     * @returns True if vector was removed
     */
    remove(id: string): boolean;
    /**
     * Force reindexing of quantized data
     */
    reindex(): void;
    /**
     * Search for nearest neighbors using asymmetric distance computation
     *
     * @param query - Query vector (full precision)
     * @param k - Number of neighbors to return
     * @returns Array of search results sorted by distance
     */
    search(query: Float32Array, k: number): SearchResult[];
    /**
     * Precompute distance tables for asymmetric distance computation
     * This enables efficient lookup during search
     *
     * @param query - Query vector
     * @returns Distance lookup tables
     */
    private computeDistanceTables;
    /**
     * Compute asymmetric distance using precomputed tables
     *
     * @param index - Vector index
     * @param tables - Precomputed distance tables
     * @returns Squared distance
     */
    private computeADCDistance;
    /**
     * Get a vector by ID (reconstructed from quantized form)
     *
     * @param id - Vector ID
     * @returns Reconstructed vector or null if not found
     */
    get(id: string): Float32Array | null;
    /**
     * Check if a vector exists in the store
     *
     * @param id - Vector ID
     * @returns True if vector exists
     */
    has(id: string): boolean;
    /**
     * Get the number of vectors in the store
     */
    get size(): number;
    /**
     * Get memory usage statistics
     */
    getMemoryStats(): {
        rawBytes: number;
        quantizedBytes: number;
        compressionRatio: number;
    };
    /**
     * Clear all vectors from the store
     */
    clear(): void;
    /**
     * Export the store for serialization
     */
    export(): {
        config: Required<QuantizationConfig>;
        ids: string[];
        quantized: QuantizedVectors | null;
        codebooks?: Float32Array[];
    };
}
/**
 * Calculate the theoretical compression ratio for a quantization type
 *
 * @param type - Quantization type
 * @param dimension - Vector dimension
 * @param numSubvectors - Number of subvectors for product quantization
 * @returns Compression ratio (original size / compressed size)
 */
export declare function calculateCompressionRatio(type: QuantizationType, dimension: number, numSubvectors?: number): number;
/**
 * Estimate memory savings for a given number of vectors
 *
 * @param type - Quantization type
 * @param dimension - Vector dimension
 * @param numVectors - Number of vectors
 * @param numSubvectors - Number of subvectors for product quantization
 * @returns Memory savings in bytes
 */
export declare function estimateMemorySavings(type: QuantizationType, dimension: number, numVectors: number, numSubvectors?: number): {
    originalBytes: number;
    compressedBytes: number;
    savedBytes: number;
    savedPercentage: number;
};
//# sourceMappingURL=Quantization.d.ts.map