/**
 * Attention Module Fallbacks
 *
 * Since @ruvector/attention is completely broken, provide JavaScript fallbacks
 * Performance will be slower but functionality will work
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
//# sourceMappingURL=attention-fallbacks.d.ts.map