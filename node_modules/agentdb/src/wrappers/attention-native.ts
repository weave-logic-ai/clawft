/**
 * Native Attention Wrappers
 *
 * Properly wraps @ruvector/attention native Rust implementations
 * with TypedArray conversions and proper error handling
 */

import * as nativeAttention from '@ruvector/attention';

export interface AttentionConfig {
  hiddenDim: number;
  numHeads?: number;
  dropoutRate?: number;
  useFlash?: boolean;
}

/**
 * Convert regular array to Float32Array if needed
 */
function toFloat32Array(input: number[] | Float32Array): Float32Array {
  if (input instanceof Float32Array) {
    return input;
  }
  return new Float32Array(input);
}

/**
 * Convert Float32Array to regular array for JS compatibility
 */
function toArray(input: Float32Array): number[] {
  return Array.from(input);
}

/**
 * Native Multi-Head Attention (uses Rust implementation)
 *
 * This wrapper properly converts between JavaScript arrays and TypedArrays
 * required by the native Rust implementation.
 */
export class MultiHeadAttention {
  private nativeInstance: any;
  private hiddenDim: number;
  private numHeads: number;

  constructor(config: AttentionConfig) {
    this.hiddenDim = config.hiddenDim;
    this.numHeads = config.numHeads || 8;

    try {
      // Create native Rust instance
      this.nativeInstance = new nativeAttention.MultiHeadAttention(
        this.hiddenDim,
        this.numHeads
      );
    } catch (error: any) {
      throw new Error(`Failed to initialize native MultiHeadAttention: ${error.message}`);
    }
  }

  /**
   * Forward pass using native Rust implementation
   */
  forward(
    query: number[] | Float32Array,
    key: number[] | Float32Array,
    value: number[] | Float32Array,
    mask?: number[] | Float32Array
  ): { output: number[]; attentionWeights: number[][] } {
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
    } catch (error: any) {
      throw new Error(`MultiHeadAttention forward failed: ${error.message}`);
    }
  }

  /**
   * Get dimensions
   */
  get dim(): number {
    return this.nativeInstance.dim;
  }

  get headDim(): number {
    return this.nativeInstance.headDim;
  }
}

/**
 * Native Flash Attention (uses Rust implementation)
 *
 * Memory-efficient attention with tiling/chunking
 */
export class FlashAttention {
  private nativeInstance: any;
  private hiddenDim: number;

  constructor(config: AttentionConfig) {
    this.hiddenDim = config.hiddenDim;

    try {
      this.nativeInstance = new nativeAttention.FlashAttention(this.hiddenDim);
    } catch (error: any) {
      throw new Error(`Failed to initialize native FlashAttention: ${error.message}`);
    }
  }

  /**
   * Forward pass with batch support
   */
  forward(
    query: number[][] | Float32Array[],
    key: number[][] | Float32Array[],
    value: number[][] | Float32Array[],
    numHeads: number = 8
  ): { output: number[][]; attentionScores: number[][] } {
    try {
      // Convert batch to Float32Array
      const q = query.map(toFloat32Array);
      const k = key.map(toFloat32Array);
      const v = value.map(toFloat32Array);

      // Call native compute
      const result = this.nativeInstance.compute(q, k, v, numHeads);

      // Convert result back
      return {
        output: result.map((r: Float32Array) => toArray(r)),
        attentionScores: [[]]
      };
    } catch (error: any) {
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
  private nativeInstance: any;
  private hiddenDim: number;

  constructor(config: AttentionConfig) {
    this.hiddenDim = config.hiddenDim;

    try {
      // LinearAttention constructor: (hiddenDim, seqLen)
      // We'll use hiddenDim for both since seqLen varies per forward call
      this.nativeInstance = new nativeAttention.LinearAttention(
        this.hiddenDim,
        this.hiddenDim
      );
    } catch (error: any) {
      throw new Error(`Failed to initialize native LinearAttention: ${error.message}`);
    }
  }

  forward(
    query: number[][] | Float32Array[],
    key: number[][] | Float32Array[],
    value: number[][] | Float32Array[]
  ): { output: number[][] } {
    try {
      const q = query.map(toFloat32Array);
      const k = key.map(toFloat32Array);
      const v = value.map(toFloat32Array);

      const result = this.nativeInstance.compute(q, k, v);

      return {
        output: result.map((r: Float32Array) => toArray(r))
      };
    } catch (error: any) {
      throw new Error(`LinearAttention forward failed: ${error.message}`);
    }
  }
}

/**
 * Native Hyperbolic Attention (uses Rust implementation)
 */
export class HyperbolicAttention {
  private nativeInstance: any;
  private hiddenDim: number;

  constructor(config: AttentionConfig) {
    this.hiddenDim = config.hiddenDim;

    try {
      this.nativeInstance = new nativeAttention.HyperbolicAttention(this.hiddenDim);
    } catch (error: any) {
      throw new Error(`Failed to initialize native HyperbolicAttention: ${error.message}`);
    }
  }

  forward(
    query: number[] | Float32Array,
    key: number[] | Float32Array,
    value: number[] | Float32Array
  ): { output: number[]; distance: number } {
    try {
      const q = toFloat32Array(query);
      const k = toFloat32Array(key);
      const v = toFloat32Array(value);

      const result = this.nativeInstance.compute(q, k, v);

      return {
        output: toArray(result.output),
        distance: result.distance
      };
    } catch (error: any) {
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
  private nativeInstance: any;
  private hiddenDim: number;
  private numExperts: number;

  constructor(config: AttentionConfig & { numExperts?: number }) {
    this.hiddenDim = config.hiddenDim;
    this.numExperts = config.numExperts || 4;

    try {
      // MoEAttention takes a config object
      this.nativeInstance = new nativeAttention.MoEAttention({
        dim: this.hiddenDim,
        numExperts: this.numExperts,
        topK: 2
      });
    } catch (error: any) {
      throw new Error(`Failed to initialize native MoEAttention: ${error.message}`);
    }
  }

  forward(
    query: number[] | Float32Array,
    key: number[] | Float32Array,
    value: number[] | Float32Array,
    topK: number = 2
  ): { output: number[]; expertWeights: number[] } {
    try {
      const q = toFloat32Array(query);
      const k = toFloat32Array(key);
      const v = toFloat32Array(value);

      const result = this.nativeInstance.compute(q, k, v, topK);

      return {
        output: toArray(result.output),
        expertWeights: toArray(result.expertWeights)
      };
    } catch (error: any) {
      throw new Error(`MoEAttention forward failed: ${error.message}`);
    }
  }
}

/**
 * Scaled Dot-Product Attention (pure JavaScript, always works)
 */
export function scaledDotProductAttention(
  query: number[],
  key: number[],
  value: number[],
  mask?: number[]
): { output: number[]; weights: number[] } {
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
export function isNativeAttentionAvailable(): boolean {
  try {
    const test = new nativeAttention.MultiHeadAttention(128, 4);
    return true;
  } catch {
    return false;
  }
}

/**
 * Factory function to create appropriate attention module
 */
export function createAttention(
  type: 'multi-head' | 'flash' | 'linear' | 'hyperbolic' | 'moe',
  config: AttentionConfig
): MultiHeadAttention | FlashAttention | LinearAttention | HyperbolicAttention | MoEAttention {
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
