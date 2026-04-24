/**
 * Native Attention Wrappers
 *
 * Properly wraps @ruvector/attention native Rust implementations
 * with TypedArray conversions and proper error handling
 */
import * as nativeAttention from '@ruvector/attention';
/**
 * Convert regular array to Float32Array if needed
 */
function toFloat32Array(input) {
    if (input instanceof Float32Array) {
        return input;
    }
    return new Float32Array(input);
}
/**
 * Convert Float32Array to regular array for JS compatibility
 */
function toArray(input) {
    return Array.from(input);
}
/**
 * Native Multi-Head Attention (uses Rust implementation)
 *
 * This wrapper properly converts between JavaScript arrays and TypedArrays
 * required by the native Rust implementation.
 */
export class MultiHeadAttention {
    nativeInstance;
    hiddenDim;
    numHeads;
    constructor(config) {
        this.hiddenDim = config.hiddenDim;
        this.numHeads = config.numHeads || 8;
        try {
            // Create native Rust instance
            this.nativeInstance = new nativeAttention.MultiHeadAttention(this.hiddenDim, this.numHeads);
        }
        catch (error) {
            throw new Error(`Failed to initialize native MultiHeadAttention: ${error.message}`);
        }
    }
    /**
     * Forward pass using native Rust implementation
     */
    forward(query, key, value, mask) {
        try {
            // Convert to Float32Array for native code
            const q = toFloat32Array(query);
            const k = toFloat32Array(key);
            const v = toFloat32Array(value);
            const m = mask ? toFloat32Array(mask) : undefined;
            // Call native compute method
            const result = this.nativeInstance.compute(q, k, v, m);
            // Convert result back to regular arrays
            return {
                output: toArray(result),
                attentionWeights: [[]] // Native doesn't return weights separately
            };
        }
        catch (error) {
            throw new Error(`MultiHeadAttention forward failed: ${error.message}`);
        }
    }
    /**
     * Get dimensions
     */
    get dim() {
        return this.nativeInstance.dim;
    }
    get headDim() {
        return this.nativeInstance.headDim;
    }
}
/**
 * Native Flash Attention (uses Rust implementation)
 *
 * Memory-efficient attention with tiling/chunking
 */
export class FlashAttention {
    nativeInstance;
    hiddenDim;
    constructor(config) {
        this.hiddenDim = config.hiddenDim;
        try {
            this.nativeInstance = new nativeAttention.FlashAttention(this.hiddenDim);
        }
        catch (error) {
            throw new Error(`Failed to initialize native FlashAttention: ${error.message}`);
        }
    }
    /**
     * Forward pass with batch support
     */
    forward(query, key, value, numHeads = 8) {
        try {
            // Convert batch to Float32Array
            const q = query.map(toFloat32Array);
            const k = key.map(toFloat32Array);
            const v = value.map(toFloat32Array);
            // Call native compute
            const result = this.nativeInstance.compute(q, k, v, numHeads);
            // Convert result back
            return {
                output: result.map((r) => toArray(r)),
                attentionScores: [[]]
            };
        }
        catch (error) {
            throw new Error(`FlashAttention forward failed: ${error.message}`);
        }
    }
}
/**
 * Native Linear Attention (uses Rust implementation)
 *
 * O(n) complexity approximation of attention
 */
export class LinearAttention {
    nativeInstance;
    hiddenDim;
    constructor(config) {
        this.hiddenDim = config.hiddenDim;
        try {
            // LinearAttention constructor: (hiddenDim, seqLen)
            // We'll use hiddenDim for both since seqLen varies per forward call
            this.nativeInstance = new nativeAttention.LinearAttention(this.hiddenDim, this.hiddenDim);
        }
        catch (error) {
            throw new Error(`Failed to initialize native LinearAttention: ${error.message}`);
        }
    }
    forward(query, key, value) {
        try {
            const q = query.map(toFloat32Array);
            const k = key.map(toFloat32Array);
            const v = value.map(toFloat32Array);
            const result = this.nativeInstance.compute(q, k, v);
            return {
                output: result.map((r) => toArray(r))
            };
        }
        catch (error) {
            throw new Error(`LinearAttention forward failed: ${error.message}`);
        }
    }
}
/**
 * Native Hyperbolic Attention (uses Rust implementation)
 */
export class HyperbolicAttention {
    nativeInstance;
    hiddenDim;
    constructor(config) {
        this.hiddenDim = config.hiddenDim;
        try {
            this.nativeInstance = new nativeAttention.HyperbolicAttention(this.hiddenDim);
        }
        catch (error) {
            throw new Error(`Failed to initialize native HyperbolicAttention: ${error.message}`);
        }
    }
    forward(query, key, value) {
        try {
            const q = toFloat32Array(query);
            const k = toFloat32Array(key);
            const v = toFloat32Array(value);
            const result = this.nativeInstance.compute(q, k, v);
            return {
                output: toArray(result.output),
                distance: result.distance
            };
        }
        catch (error) {
            throw new Error(`HyperbolicAttention forward failed: ${error.message}`);
        }
    }
}
/**
 * Native MoE Attention (uses Rust implementation)
 *
 * Mixture of Experts with top-k routing
 */
export class MoEAttention {
    nativeInstance;
    hiddenDim;
    numExperts;
    constructor(config) {
        this.hiddenDim = config.hiddenDim;
        this.numExperts = config.numExperts || 4;
        try {
            this.nativeInstance = new nativeAttention.MoEAttention(this.hiddenDim, this.numExperts);
        }
        catch (error) {
            throw new Error(`Failed to initialize native MoEAttention: ${error.message}`);
        }
    }
    forward(query, key, value, topK = 2) {
        try {
            const q = toFloat32Array(query);
            const k = toFloat32Array(key);
            const v = toFloat32Array(value);
            const result = this.nativeInstance.compute(q, k, v, topK);
            return {
                output: toArray(result.output),
                expertWeights: toArray(result.expertWeights)
            };
        }
        catch (error) {
            throw new Error(`MoEAttention forward failed: ${error.message}`);
        }
    }
}
/**
 * Scaled Dot-Product Attention (pure JavaScript, always works)
 */
export function scaledDotProductAttention(query, key, value, mask) {
    const dk = query.length;
    // Compute attention scores: Q · K^T / sqrt(dk)
    let score = 0;
    for (let i = 0; i < dk; i++) {
        score += query[i] * key[i];
    }
    score /= Math.sqrt(dk);
    // Apply mask if provided
    if (mask && mask[0] === 0) {
        score = -Infinity;
    }
    // Softmax (single score version)
    const expScore = Math.exp(score);
    const weight = expScore;
    // Weighted value
    const output = value.map(v => v * weight);
    return { output, weights: [weight] };
}
/**
 * Check if native attention is available
 */
export function isNativeAttentionAvailable() {
    try {
        const test = new nativeAttention.MultiHeadAttention(128, 4);
        return true;
    }
    catch {
        return false;
    }
}
/**
 * Factory function to create appropriate attention module
 */
export function createAttention(type, config) {
    switch (type) {
        case 'multi-head':
            return new MultiHeadAttention(config);
        case 'flash':
            return new FlashAttention(config);
        case 'linear':
            return new LinearAttention(config);
        case 'hyperbolic':
            return new HyperbolicAttention(config);
        case 'moe':
            return new MoEAttention(config);
        default:
            return new MultiHeadAttention(config);
    }
}
//# sourceMappingURL=attention-native.js.map