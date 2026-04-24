/**
 * SIMD-Accelerated Vector Operations for AgentDB
 *
 * High-performance vector operations with WebAssembly SIMD acceleration.
 * Provides 2-8x speedup over standard JavaScript implementations through:
 * - 8x loop unrolling with separate accumulators for Instruction-Level Parallelism (ILP)
 * - Pre-allocated buffer pools to minimize GC pressure
 * - SIMD detection and graceful fallback to optimized scalar code
 * - Batch operations for cache-friendly memory access patterns
 *
 * @module simd-vector-ops
 */
/** @inline Maximum allowed vector dimension to prevent DoS via large allocations */
export declare const MAX_VECTOR_DIMENSION = 4096;
/** @inline Maximum batch size for optimal cache utilization */
export declare const MAX_BATCH_SIZE = 10000;
/** @inline Default cache size for embedding storage */
export declare const DEFAULT_CACHE_SIZE = 10000;
/**
 * Configuration options for SIMD vector operations
 */
export interface SIMDConfig {
    /** Enable SIMD acceleration (auto-detected if not specified) */
    enableSIMD?: boolean;
    /** Size of pre-allocated buffer pool */
    bufferPoolSize?: number;
    /** Default vector dimension for buffer allocation */
    defaultDimension?: number;
    /** Enable performance logging */
    enableLogging?: boolean;
}
/**
 * Result of batch similarity computation
 */
export interface BatchSimilarityResult {
    /** Index in the original vector array */
    index: number;
    /** Cosine similarity score [-1, 1] */
    similarity: number;
    /** Euclidean distance (L2 norm) */
    distance?: number;
}
/**
 * Statistics about SIMD operations
 */
export interface SIMDStats {
    /** Whether SIMD is available and enabled */
    simdEnabled: boolean;
    /** Number of vectors processed */
    vectorsProcessed: number;
    /** Total operations performed */
    operationsCount: number;
    /** Buffers currently in pool */
    buffersInPool: number;
    /** Peak buffer usage */
    peakBufferUsage: number;
}
/**
 * Detect WebAssembly SIMD support
 *
 * This function caches the result after first call for performance.
 * The detection is done by validating a minimal WASM module containing
 * SIMD instructions.
 *
 * @returns true if SIMD is supported, false otherwise
 */
export declare function detectSIMDSupport(): boolean;
/**
 * Alternative SIMD detection using the simpler v128.const check
 * This is the same check used in WASMVectorSearch for consistency
 */
export declare function detectSIMDSupportLegacy(): boolean;
/**
 * Calculate cosine similarity with 8x loop unrolling and ILP
 *
 * Uses 8 separate accumulators to maximize instruction-level parallelism.
 * Modern CPUs can execute multiple independent operations in parallel,
 * and separate accumulators prevent data dependencies between iterations.
 *
 * Performance characteristics:
 * - 8x unrolling reduces loop overhead by 87.5%
 * - Separate accumulators enable out-of-order execution
 * - Final reduction combines accumulators efficiently
 *
 * @param a - First vector (Float32Array)
 * @param b - Second vector (Float32Array)
 * @returns Cosine similarity in range [-1, 1]
 * @throws Error if vectors have different lengths
 */
export declare function cosineSimilaritySIMD(a: Float32Array, b: Float32Array): number;
/**
 * Calculate Euclidean (L2) distance with 8x loop unrolling
 *
 * Uses the same ILP optimization as cosine similarity.
 *
 * @param a - First vector (Float32Array)
 * @param b - Second vector (Float32Array)
 * @returns Euclidean distance (non-negative)
 * @throws Error if vectors have different lengths
 */
export declare function euclideanDistanceSIMD(a: Float32Array, b: Float32Array): number;
/**
 * Calculate squared Euclidean distance (faster than euclideanDistanceSIMD)
 *
 * Useful when you only need to compare distances (skip the sqrt).
 *
 * @param a - First vector
 * @param b - Second vector
 * @returns Squared Euclidean distance
 */
export declare function euclideanDistanceSquaredSIMD(a: Float32Array, b: Float32Array): number;
/**
 * Calculate dot product with 8x loop unrolling
 *
 * @param a - First vector
 * @param b - Second vector
 * @returns Dot product
 */
export declare function dotProductSIMD(a: Float32Array, b: Float32Array): number;
/**
 * Calculate L2 norm (magnitude) of a vector with 8x unrolling
 *
 * @param v - Input vector
 * @returns L2 norm
 */
export declare function l2NormSIMD(v: Float32Array): number;
/**
 * Normalize a vector to unit length (in-place)
 *
 * @param v - Vector to normalize (modified in-place)
 * @returns The same vector, normalized
 */
export declare function normalizeVectorInPlace(v: Float32Array): Float32Array;
/**
 * Create a normalized copy of a vector
 *
 * @param v - Input vector
 * @returns New normalized vector
 */
export declare function normalizeVector(v: Float32Array): Float32Array;
/**
 * Batch cosine similarity calculation with early termination optimization
 *
 * Efficiently computes similarities between a query vector and multiple
 * candidate vectors. Uses cache-friendly memory access patterns.
 *
 * @inline V8 optimization hint - hot path function
 * @param query - Query vector
 * @param vectors - Array of candidate vectors
 * @param options - Optional configuration
 * @returns Array of similarity scores (same order as input vectors)
 */
export declare function batchCosineSimilarity(query: Float32Array, vectors: Float32Array[], options?: {
    /** Return only top-k results (sorted by similarity) */
    topK?: number;
    /** Minimum similarity threshold */
    threshold?: number;
    /** Early termination when this many perfect matches found */
    earlyTerminationThreshold?: number;
}): BatchSimilarityResult[];
/**
 * Batch Euclidean distance calculation with early termination
 *
 * @inline V8 optimization hint - hot path function
 * @param query - Query vector
 * @param vectors - Array of candidate vectors
 * @param options - Optional configuration
 * @returns Array of distance results (same order as input vectors)
 */
export declare function batchEuclideanDistance(query: Float32Array, vectors: Float32Array[], options?: {
    /** Return only top-k results (sorted by distance, ascending) */
    topK?: number;
    /** Maximum distance threshold */
    maxDistance?: number;
    /** Use squared distance (faster, useful for comparisons) */
    squared?: boolean;
    /** Early termination distance threshold */
    earlyTerminationDistance?: number;
}): BatchSimilarityResult[];
/**
 * SIMD-accelerated vector operations manager
 *
 * Provides a stateful wrapper around vector operations with:
 * - Automatic SIMD detection
 * - Pre-allocated buffer pools
 * - Operation statistics tracking
 * - Configurable behavior
 */
export declare class SIMDVectorOps {
    private config;
    private simdEnabled;
    private bufferPool;
    private stats;
    constructor(config?: SIMDConfig);
    /**
     * Calculate cosine similarity between two vectors
     */
    cosineSimilarity(a: Float32Array, b: Float32Array): number;
    /**
     * Calculate Euclidean distance between two vectors
     */
    euclideanDistance(a: Float32Array, b: Float32Array): number;
    /**
     * Calculate squared Euclidean distance (faster for comparisons)
     */
    euclideanDistanceSquared(a: Float32Array, b: Float32Array): number;
    /**
     * Calculate dot product
     */
    dotProduct(a: Float32Array, b: Float32Array): number;
    /**
     * Calculate L2 norm
     */
    l2Norm(v: Float32Array): number;
    /**
     * Normalize a vector (creates a copy)
     */
    normalize(v: Float32Array): Float32Array;
    /**
     * Normalize a vector in place
     */
    normalizeInPlace(v: Float32Array): Float32Array;
    /**
     * Batch cosine similarity
     */
    batchCosineSimilarity(query: Float32Array, vectors: Float32Array[], options?: {
        topK?: number;
        threshold?: number;
    }): BatchSimilarityResult[];
    /**
     * Batch Euclidean distance
     */
    batchEuclideanDistance(query: Float32Array, vectors: Float32Array[], options?: {
        topK?: number;
        maxDistance?: number;
        squared?: boolean;
    }): BatchSimilarityResult[];
    /**
     * Acquire a temporary buffer from the pool
     */
    acquireBuffer(size?: number): Float32Array;
    /**
     * Release a buffer back to the pool
     */
    releaseBuffer(buffer: Float32Array): void;
    /**
     * Get operation statistics
     */
    getStats(): SIMDStats;
    /**
     * Reset statistics
     */
    resetStats(): void;
    /**
     * Check if SIMD is enabled
     */
    isSIMDEnabled(): boolean;
    /**
     * Clear buffer pools
     */
    clearBufferPool(): void;
}
/**
 * Create a Float32Array from various input types
 *
 * @param data - Input data (array of numbers, ArrayBuffer, or existing Float32Array)
 * @returns Float32Array
 */
export declare function toFloat32Array(data: number[] | ArrayBuffer | Float32Array | ArrayLike<number>): Float32Array;
/**
 * Create random unit vector for testing
 *
 * @param dimension - Vector dimension
 * @returns Normalized random vector
 */
export declare function randomUnitVector(dimension: number): Float32Array;
/**
 * Vector addition with 8x unrolling
 *
 * @param a - First vector
 * @param b - Second vector
 * @param out - Optional output buffer (creates new if not provided)
 * @returns Result vector (a + b)
 */
export declare function vectorAdd(a: Float32Array, b: Float32Array, out?: Float32Array): Float32Array;
/**
 * Vector subtraction with 8x unrolling
 *
 * @param a - First vector
 * @param b - Second vector
 * @param out - Optional output buffer
 * @returns Result vector (a - b)
 */
export declare function vectorSub(a: Float32Array, b: Float32Array, out?: Float32Array): Float32Array;
/**
 * Scalar multiplication with 8x unrolling
 *
 * @param v - Input vector
 * @param scalar - Scalar multiplier
 * @param out - Optional output buffer
 * @returns Result vector (v * scalar)
 */
export declare function vectorScale(v: Float32Array, scalar: number, out?: Float32Array): Float32Array;
/**
 * Default SIMD vector operations instance
 *
 * Pre-configured with sensible defaults. For custom configuration,
 * create a new SIMDVectorOps instance.
 */
export declare const defaultSIMDOps: SIMDVectorOps;
declare const _default: {
    cosineSimilaritySIMD: typeof cosineSimilaritySIMD;
    euclideanDistanceSIMD: typeof euclideanDistanceSIMD;
    euclideanDistanceSquaredSIMD: typeof euclideanDistanceSquaredSIMD;
    dotProductSIMD: typeof dotProductSIMD;
    l2NormSIMD: typeof l2NormSIMD;
    normalizeVector: typeof normalizeVector;
    normalizeVectorInPlace: typeof normalizeVectorInPlace;
    batchCosineSimilarity: typeof batchCosineSimilarity;
    batchEuclideanDistance: typeof batchEuclideanDistance;
    vectorAdd: typeof vectorAdd;
    vectorSub: typeof vectorSub;
    vectorScale: typeof vectorScale;
    toFloat32Array: typeof toFloat32Array;
    randomUnitVector: typeof randomUnitVector;
    detectSIMDSupport: typeof detectSIMDSupport;
    detectSIMDSupportLegacy: typeof detectSIMDSupportLegacy;
    SIMDVectorOps: typeof SIMDVectorOps;
    defaultSIMDOps: SIMDVectorOps;
};
export default _default;
//# sourceMappingURL=simd-vector-ops.d.ts.map