/**
 * Attention Module Fallbacks
 *
 * Since @ruvector/attention is completely broken, provide JavaScript fallbacks
 * Performance will be slower but functionality will work
 *
 * Optimized version includes:
 * - SIMD-style loop unrolling (8x with separate accumulators)
 * - Pre-allocated buffer pools for intermediate results
 * - Numerically stable fused softmax operations
 * - TypedArray optimizations (Float32Array)
 * - Batch processing capabilities
 * - Flash attention with improved tiling strategy
 */
export interface AttentionConfig {
    hiddenDim: number;
    numHeads?: number;
    dropoutRate?: number;
    useFlash?: boolean;
}
/**
 * Scaled Dot-Product Attention
 * The core attention mechanism
 */
export declare function scaledDotProductAttention(query: number[], key: number[], value: number[], mask?: number[]): {
    output: number[];
    weights: number[];
};
/**
 * Optimized scaled dot-product attention using TypedArrays and SIMD-style ops
 */
export declare function scaledDotProductAttentionOptimized(query: Float32Array, key: Float32Array, value: Float32Array, mask?: Float32Array | null): {
    output: Float32Array;
    weights: Float32Array;
};
/**
 * Batch scaled dot-product attention for processing multiple queries at once
 * Processes multiple Q-K-V triplets efficiently
 */
export declare function batchScaledDotProductAttention(queries: Float32Array[], keys: Float32Array[], values: Float32Array[], masks?: (Float32Array | null)[]): {
    outputs: Float32Array[];
    weights: Float32Array[];
};
/**
 * Full attention computation over sequences (Q, K, V matrices)
 * Returns attention output and weights for all query positions
 */
export declare function batchSequenceAttention(queries: Float32Array, // [seqLen * dim] flattened
keys: Float32Array, // [seqLen * dim] flattened
values: Float32Array, // [seqLen * dim] flattened
seqLen: number, dim: number, mask?: Float32Array): {
    output: Float32Array;
    weights: Float32Array;
};
/**
 * Multi-Head Attention (JavaScript fallback)
 *
 * Replaces broken @ruvector/attention.multiHeadAttention
 */
export declare class MultiHeadAttention {
    private numHeads;
    private hiddenDim;
    private headDim;
    private queryWeights;
    private keyWeights;
    private valueWeights;
    private outputWeights;
    constructor(config: AttentionConfig);
    private initializeWeights;
    private initializeOutputWeights;
    forward(query: number[], key: number[], value: number[], mask?: number[]): {
        output: number[];
        attentionWeights: number[][];
    };
    private project;
}
/**
 * Optimized Multi-Head Attention using TypedArrays and pre-allocated buffers
 *
 * Improvements over original:
 * - Float32Array instead of number[]
 * - Pre-allocated weight matrices in contiguous memory
 * - 8x loop unrolling for projections
 * - Buffer pool for intermediate results
 * - Batch processing support
 */
export declare class MultiHeadAttentionOptimized {
    private numHeads;
    private hiddenDim;
    private headDim;
    private queryWeights;
    private keyWeights;
    private valueWeights;
    private outputWeights;
    private headBuffer;
    private projBuffer;
    private concatBuffer;
    constructor(config: AttentionConfig);
    private initializeProjectionWeights;
    private initializeOutputWeights;
    /**
     * Forward pass with optimized TypedArray operations
     */
    forward(query: Float32Array, key: Float32Array, value: Float32Array, mask?: Float32Array | null): {
        output: Float32Array;
        attentionWeights: Float32Array[];
    };
    /**
     * Batch forward pass for multiple inputs
     */
    batchForward(queries: Float32Array[], keys: Float32Array[], values: Float32Array[], masks?: (Float32Array | null)[]): {
        outputs: Float32Array[];
        attentionWeights: Float32Array[][];
    };
    /**
     * Optimized projection using contiguous weight storage and 8x unrolling
     */
    private projectOptimized;
    /**
     * Release buffers back to pool (call when done with outputs)
     */
    releaseBuffer(buffer: Float32Array): void;
    /**
     * Get weight matrices (for serialization/loading)
     */
    getWeights(): {
        query: Float32Array;
        key: Float32Array;
        value: Float32Array;
        output: Float32Array;
    };
    /**
     * Set weight matrices (for loading pre-trained weights)
     */
    setWeights(weights: {
        query: Float32Array;
        key: Float32Array;
        value: Float32Array;
        output: Float32Array;
    }): void;
}
/**
 * Flash Attention (optimized fallback)
 *
 * Replaces broken @ruvector/attention.flashAttention
 * Uses tiling/chunking for better memory efficiency
 */
export declare class FlashAttention {
    private hiddenDim;
    private blockSize;
    constructor(config: AttentionConfig);
    forward(query: number[][], key: number[][], value: number[][], numHeads?: number): {
        output: number[][];
        attentionScores: number[][];
    };
}
/**
 * Flash Attention Configuration
 */
export interface FlashAttentionConfig extends AttentionConfig {
    blockSizeQ?: number;
    blockSizeKV?: number;
    causal?: boolean;
}
/**
 * Optimized Flash Attention using TypedArrays and improved tiling strategy
 *
 * Improvements over original:
 * - Separate Q and KV block sizes for better memory usage
 * - Online softmax computation (memory efficient)
 * - Pre-allocated buffers for all intermediate results
 * - 8x loop unrolling for score computation
 * - Causal masking support
 * - Backward pass placeholder for gradient computation
 */
export declare class FlashAttentionOptimized {
    private hiddenDim;
    private blockSizeQ;
    private blockSizeKV;
    private causal;
    private scoresBuffer;
    private expBuffer;
    private outputBuffer;
    private maxBuffer;
    private sumBuffer;
    constructor(config: FlashAttentionConfig);
    /**
     * Forward pass with optimized tiling strategy
     * Uses online softmax for memory efficiency
     */
    forward(query: Float32Array, // [seqLen * dim] flattened
    key: Float32Array, // [seqLen * dim] flattened
    value: Float32Array, // [seqLen * dim] flattened
    seqLen: number, dim: number, numHeads?: number): {
        output: Float32Array;
        attentionScores: Float32Array;
    };
    /**
     * Compute attention scores for a block pair with 8x unrolling
     */
    private computeBlockScores;
    /**
     * Online softmax update with output accumulation
     * This is the core of flash attention's memory efficiency
     */
    private updateOnlineSoftmax;
    /**
     * Batch forward for processing multiple sequences
     */
    batchForward(queries: Float32Array[], keys: Float32Array[], values: Float32Array[], seqLens: number[], dim: number, numHeads?: number): {
        outputs: Float32Array[];
        attentionScores: Float32Array[];
    };
    /**
     * Backward pass for gradient computation
     * Computes dQ, dK, dV from dOutput using the attention scores from forward pass
     *
     * Forward pass recap:
     *   S = Q @ K^T / sqrt(d)     (scaled dot-product scores)
     *   A = softmax(S)            (attention weights)
     *   O = A @ V                 (output)
     *
     * Backward pass:
     *   dV = A^T @ dO             (gradient w.r.t. values)
     *   dA = dO @ V^T             (gradient through output)
     *   dS = A * (dA - sum(dA * A, axis=-1, keepdims=True))  (softmax backward)
     *   dS = dS / sqrt(d)         (apply scaling)
     *   dQ = dS @ K               (gradient w.r.t. queries)
     *   dK = dS^T @ Q             (gradient w.r.t. keys)
     */
    backward(dOutput: Float32Array, query: Float32Array, key: Float32Array, value: Float32Array, attentionScores: Float32Array, seqLen: number, dim: number): {
        dQuery: Float32Array;
        dKey: Float32Array;
        dValue: Float32Array;
    };
    /**
     * Release buffers back to pool
     */
    releaseBuffer(buffer: Float32Array): void;
}
/**
 * Batch multi-head attention for processing multiple queries efficiently
 * Combines optimized MHA with batch processing
 */
export declare function batchMultiHeadAttention(attention: MultiHeadAttentionOptimized, queries: Float32Array[], keys: Float32Array[], values: Float32Array[], masks?: (Float32Array | null)[]): {
    outputs: Float32Array[];
    attentionWeights: Float32Array[][];
};
/**
 * Linear Attention (fallback)
 *
 * O(n) complexity approximation of attention
 */
export declare class LinearAttention {
    private hiddenDim;
    private featureMap;
    constructor(config: AttentionConfig);
    forward(query: number[][], key: number[][], value: number[][]): {
        output: number[][];
    };
}
/**
 * Hyperbolic Attention (simplified fallback)
 *
 * Approximation using hyperbolic geometry
 */
export declare class HyperbolicAttention {
    private hiddenDim;
    private curvature;
    constructor(config: AttentionConfig);
    forward(query: number[], key: number[], value: number[]): {
        output: number[];
        distance: number;
    };
    private hyperbolicDistance;
}
/**
 * MoE (Mixture of Experts) Attention (fallback)
 *
 * Routes to different expert attention modules
 */
export declare class MoEAttention {
    private experts;
    private numExperts;
    private gatingWeights;
    constructor(config: AttentionConfig & {
        numExperts?: number;
    });
    forward(query: number[], key: number[], value: number[], topK?: number): {
        output: number[];
        expertWeights: number[];
    };
}
/**
 * Check if native attention is available
 */
export declare function isNativeAttentionAvailable(): boolean;
/**
 * Factory function to create appropriate attention module
 */
export declare function createAttention(type: 'multi-head' | 'flash' | 'linear' | 'hyperbolic' | 'moe', config: AttentionConfig): MultiHeadAttention | FlashAttention | LinearAttention | HyperbolicAttention | MoEAttention;
/**
 * Factory function to create optimized attention modules
 */
export declare function createAttentionOptimized(type: 'multi-head' | 'flash', config: AttentionConfig | FlashAttentionConfig): MultiHeadAttentionOptimized | FlashAttentionOptimized;
/**
 * Convert number[] to Float32Array
 */
export declare function toFloat32Array(arr: number[]): Float32Array;
/**
 * Convert Float32Array to number[]
 */
export declare function toNumberArray(arr: Float32Array): number[];
/**
 * Convert 2D number[][] to flattened Float32Array
 */
export declare function flatten2D(arr: number[][]): Float32Array;
/**
 * Unflatten Float32Array to 2D array
 */
export declare function unflatten2D(arr: Float32Array, rows: number, cols: number): number[][];
/**
 * Get the global buffer pool for external memory management
 */
export declare function getBufferPool(): {
    acquire: (size: number) => Float32Array;
    release: (buffer: Float32Array) => void;
    clear: () => void;
};
/**
 * Benchmark attention implementation
 */
export declare function benchmarkAttention(attention: MultiHeadAttentionOptimized | FlashAttentionOptimized, config: {
    seqLen: number;
    dim: number;
    iterations: number;
    numHeads?: number;
}): {
    avgTimeMs: number;
    opsPerSecond: number;
    memoryUsed: number;
};
//# sourceMappingURL=attention-fallbacks.d.ts.map