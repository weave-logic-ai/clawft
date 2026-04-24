/**
 * Native Attention Wrappers
 *
 * Properly wraps @ruvector/attention native Rust implementations
 * with TypedArray conversions and proper error handling
 */
export interface AttentionConfig {
    hiddenDim: number;
    numHeads?: number;
    dropoutRate?: number;
    useFlash?: boolean;
}
/**
 * Native Multi-Head Attention (uses Rust implementation)
 *
 * This wrapper properly converts between JavaScript arrays and TypedArrays
 * required by the native Rust implementation.
 */
export declare class MultiHeadAttention {
    private nativeInstance;
    private hiddenDim;
    private numHeads;
    constructor(config: AttentionConfig);
    /**
     * Forward pass using native Rust implementation
     */
    forward(query: number[] | Float32Array, key: number[] | Float32Array, value: number[] | Float32Array, mask?: number[] | Float32Array): {
        output: number[];
        attentionWeights: number[][];
    };
    /**
     * Get dimensions
     */
    get dim(): number;
    get headDim(): number;
}
/**
 * Native Flash Attention (uses Rust implementation)
 *
 * Memory-efficient attention with tiling/chunking
 */
export declare class FlashAttention {
    private nativeInstance;
    private hiddenDim;
    constructor(config: AttentionConfig);
    /**
     * Forward pass with batch support
     */
    forward(query: number[][] | Float32Array[], key: number[][] | Float32Array[], value: number[][] | Float32Array[], numHeads?: number): {
        output: number[][];
        attentionScores: number[][];
    };
}
/**
 * Native Linear Attention (uses Rust implementation)
 *
 * O(n) complexity approximation of attention
 */
export declare class LinearAttention {
    private nativeInstance;
    private hiddenDim;
    constructor(config: AttentionConfig);
    forward(query: number[][] | Float32Array[], key: number[][] | Float32Array[], value: number[][] | Float32Array[]): {
        output: number[][];
    };
}
/**
 * Native Hyperbolic Attention (uses Rust implementation)
 */
export declare class HyperbolicAttention {
    private nativeInstance;
    private hiddenDim;
    constructor(config: AttentionConfig);
    forward(query: number[] | Float32Array, key: number[] | Float32Array, value: number[] | Float32Array): {
        output: number[];
        distance: number;
    };
}
/**
 * Native MoE Attention (uses Rust implementation)
 *
 * Mixture of Experts with top-k routing
 */
export declare class MoEAttention {
    private nativeInstance;
    private hiddenDim;
    private numExperts;
    constructor(config: AttentionConfig & {
        numExperts?: number;
    });
    forward(query: number[] | Float32Array, key: number[] | Float32Array, value: number[] | Float32Array, topK?: number): {
        output: number[];
        expertWeights: number[];
    };
}
/**
 * Scaled Dot-Product Attention (pure JavaScript, always works)
 */
export declare function scaledDotProductAttention(query: number[], key: number[], value: number[], mask?: number[]): {
    output: number[];
    weights: number[];
};
/**
 * Check if native attention is available
 */
export declare function isNativeAttentionAvailable(): boolean;
/**
 * Factory function to create appropriate attention module
 */
export declare function createAttention(type: 'multi-head' | 'flash' | 'linear' | 'hyperbolic' | 'moe', config: AttentionConfig): MultiHeadAttention | FlashAttention | LinearAttention | HyperbolicAttention | MoEAttention;
//# sourceMappingURL=attention-native.d.ts.map