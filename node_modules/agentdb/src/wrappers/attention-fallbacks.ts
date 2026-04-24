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

// ============================================================================
// Security Constants
// ============================================================================

/** Maximum hidden dimension to prevent memory exhaustion */
const MAX_HIDDEN_DIM = 16384;

/** Maximum sequence length for attention */
const MAX_SEQ_LENGTH = 32768;

/** Maximum buffer pool size per dimension */
const MAX_BUFFER_POOL_SIZE = 64;

/** Maximum total memory for buffer pools (64MB) */
const MAX_BUFFER_POOL_MEMORY = 64 * 1024 * 1024;

/** Small epsilon for numerical stability */
const EPSILON = 1e-8;

// ============================================================================
// Validation Helpers
// ============================================================================

/**
 * Validate dimension is within safe limits
 */
function validateDimension(dim: number, name: string): void {
  if (!Number.isFinite(dim) || dim < 1 || dim > MAX_HIDDEN_DIM) {
    throw new Error(`${name} must be between 1 and ${MAX_HIDDEN_DIM}`);
  }
}

/**
 * Validate sequence length is within safe limits
 */
function validateSeqLength(len: number, name: string): void {
  if (!Number.isFinite(len) || len < 1 || len > MAX_SEQ_LENGTH) {
    throw new Error(`${name} must be between 1 and ${MAX_SEQ_LENGTH}`);
  }
}

/**
 * Validate Float32Array input
 */
function validateFloat32Array(arr: unknown, name: string): asserts arr is Float32Array {
  if (!(arr instanceof Float32Array)) {
    throw new Error(`${name} must be a Float32Array`);
  }
}

export interface AttentionConfig {
  hiddenDim: number;
  numHeads?: number;
  dropoutRate?: number;
  useFlash?: boolean;
}

// ============================================================================
// Buffer Pool Management - Pre-allocated buffers for hot paths
// ============================================================================

/**
 * Buffer pool for reusing Float32Arrays to avoid GC pressure
 */
class BufferPool {
  private pools: Map<number, Float32Array[]> = new Map();
  private maxPoolSize = MAX_BUFFER_POOL_SIZE;
  private totalMemory = 0;

  acquire(size: number): Float32Array {
    // Validate size to prevent DoS
    if (!Number.isFinite(size) || size < 0 || size > MAX_HIDDEN_DIM * MAX_SEQ_LENGTH) {
      throw new Error(`Invalid buffer size: ${size}`);
    }

    const pool = this.pools.get(size);
    if (pool && pool.length > 0) {
      return pool.pop()!;
    }
    return new Float32Array(size);
  }

  release(buffer: Float32Array): void {
    const size = buffer.length;
    const bufferBytes = size * 4; // Float32 = 4 bytes

    // Check total memory limit before adding to pool
    if (this.totalMemory + bufferBytes > MAX_BUFFER_POOL_MEMORY) {
      return; // Don't pool, let GC handle it
    }

    let pool = this.pools.get(size);
    if (!pool) {
      pool = [];
      this.pools.set(size, pool);
    }
    if (pool.length < this.maxPoolSize) {
      buffer.fill(0); // Clear for reuse
      pool.push(buffer);
      this.totalMemory += bufferBytes;
    }
  }

  clear(): void {
    this.pools.clear();
    this.totalMemory = 0;
  }
}

// Global buffer pool instance
const globalBufferPool = new BufferPool();

// ============================================================================
// SIMD-Style Optimized Operations
// ============================================================================

/**
 * 8x unrolled dot product with separate accumulators
 * Mimics SIMD behavior for better CPU pipelining
 */
function dotProduct8x(a: Float32Array, b: Float32Array, length: number): number {
  let sum0 = 0, sum1 = 0, sum2 = 0, sum3 = 0;
  let sum4 = 0, sum5 = 0, sum6 = 0, sum7 = 0;

  const unrolledLen = length - (length % 8);

  // Main 8x unrolled loop
  for (let i = 0; i < unrolledLen; i += 8) {
    sum0 += a[i] * b[i];
    sum1 += a[i + 1] * b[i + 1];
    sum2 += a[i + 2] * b[i + 2];
    sum3 += a[i + 3] * b[i + 3];
    sum4 += a[i + 4] * b[i + 4];
    sum5 += a[i + 5] * b[i + 5];
    sum6 += a[i + 6] * b[i + 6];
    sum7 += a[i + 7] * b[i + 7];
  }

  // Handle remainder
  for (let i = unrolledLen; i < length; i++) {
    sum0 += a[i] * b[i];
  }

  // Combine accumulators (in pairs to maintain precision)
  return (sum0 + sum1) + (sum2 + sum3) + (sum4 + sum5) + (sum6 + sum7);
}

/**
 * 8x unrolled matrix-vector multiplication
 * output[i] = sum_j(matrix[i][j] * vector[j])
 */
function matVecMul8x(
  matrix: Float32Array,
  vector: Float32Array,
  output: Float32Array,
  rows: number,
  cols: number
): void {
  const unrolledCols = cols - (cols % 8);

  for (let i = 0; i < rows; i++) {
    let sum0 = 0, sum1 = 0, sum2 = 0, sum3 = 0;
    let sum4 = 0, sum5 = 0, sum6 = 0, sum7 = 0;

    const rowOffset = i * cols;

    // 8x unrolled inner loop
    for (let j = 0; j < unrolledCols; j += 8) {
      const idx = rowOffset + j;
      sum0 += matrix[idx] * vector[j];
      sum1 += matrix[idx + 1] * vector[j + 1];
      sum2 += matrix[idx + 2] * vector[j + 2];
      sum3 += matrix[idx + 3] * vector[j + 3];
      sum4 += matrix[idx + 4] * vector[j + 4];
      sum5 += matrix[idx + 5] * vector[j + 5];
      sum6 += matrix[idx + 6] * vector[j + 6];
      sum7 += matrix[idx + 7] * vector[j + 7];
    }

    // Handle remainder
    for (let j = unrolledCols; j < cols; j++) {
      sum0 += matrix[rowOffset + j] * vector[j];
    }

    output[i] = (sum0 + sum1) + (sum2 + sum3) + (sum4 + sum5) + (sum6 + sum7);
  }
}

/**
 * 8x unrolled scaled add: output[i] = a[i] + scale * b[i]
 */
function scaledAdd8x(
  output: Float32Array,
  a: Float32Array,
  b: Float32Array,
  scale: number,
  length: number
): void {
  const unrolledLen = length - (length % 8);

  for (let i = 0; i < unrolledLen; i += 8) {
    output[i] = a[i] + scale * b[i];
    output[i + 1] = a[i + 1] + scale * b[i + 1];
    output[i + 2] = a[i + 2] + scale * b[i + 2];
    output[i + 3] = a[i + 3] + scale * b[i + 3];
    output[i + 4] = a[i + 4] + scale * b[i + 4];
    output[i + 5] = a[i + 5] + scale * b[i + 5];
    output[i + 6] = a[i + 6] + scale * b[i + 6];
    output[i + 7] = a[i + 7] + scale * b[i + 7];
  }

  for (let i = unrolledLen; i < length; i++) {
    output[i] = a[i] + scale * b[i];
  }
}

// ============================================================================
// Optimized Softmax Operations
// ============================================================================

/**
 * Numerically stable softmax with fused max-subtract-exp-sum operation
 * Single pass for max finding, second pass for exp and sum
 */
function softmaxInPlace(scores: Float32Array, length: number): void {
  // Validate length
  if (length <= 0 || length > scores.length) {
    throw new Error(`Invalid length: ${length}`);
  }

  // Find max (8x unrolled)
  let max0 = -Infinity, max1 = -Infinity, max2 = -Infinity, max3 = -Infinity;
  const unrolledLen = length - (length % 4);

  for (let i = 0; i < unrolledLen; i += 4) {
    if (scores[i] > max0) max0 = scores[i];
    if (scores[i + 1] > max1) max1 = scores[i + 1];
    if (scores[i + 2] > max2) max2 = scores[i + 2];
    if (scores[i + 3] > max3) max3 = scores[i + 3];
  }

  let maxVal = Math.max(max0, max1, max2, max3);
  for (let i = unrolledLen; i < length; i++) {
    if (scores[i] > maxVal) maxVal = scores[i];
  }

  // Handle edge case: all -Infinity scores
  if (!Number.isFinite(maxVal)) {
    // Uniform distribution when all scores are -Infinity
    const uniformVal = 1.0 / length;
    for (let i = 0; i < length; i++) {
      scores[i] = uniformVal;
    }
    return;
  }

  // Fused subtract-exp-sum (4x unrolled for better cache usage)
  let sum0 = 0, sum1 = 0, sum2 = 0, sum3 = 0;

  for (let i = 0; i < unrolledLen; i += 4) {
    const e0 = Math.exp(scores[i] - maxVal);
    const e1 = Math.exp(scores[i + 1] - maxVal);
    const e2 = Math.exp(scores[i + 2] - maxVal);
    const e3 = Math.exp(scores[i + 3] - maxVal);

    scores[i] = e0;
    scores[i + 1] = e1;
    scores[i + 2] = e2;
    scores[i + 3] = e3;

    sum0 += e0;
    sum1 += e1;
    sum2 += e2;
    sum3 += e3;
  }

  for (let i = unrolledLen; i < length; i++) {
    const e = Math.exp(scores[i] - maxVal);
    scores[i] = e;
    sum0 += e;
  }

  const sumTotal = sum0 + sum1 + sum2 + sum3;

  // Guard against division by zero
  if (sumTotal < EPSILON) {
    // Uniform distribution when sum is effectively zero
    const uniformVal = 1.0 / length;
    for (let i = 0; i < length; i++) {
      scores[i] = uniformVal;
    }
    return;
  }

  const invSum = 1.0 / sumTotal;

  // Normalize (4x unrolled)
  for (let i = 0; i < unrolledLen; i += 4) {
    scores[i] *= invSum;
    scores[i + 1] *= invSum;
    scores[i + 2] *= invSum;
    scores[i + 3] *= invSum;
  }

  for (let i = unrolledLen; i < length; i++) {
    scores[i] *= invSum;
  }
}

/**
 * Softmax returning new array (when in-place not desired)
 */
function softmaxOptimized(input: Float32Array): Float32Array {
  const output = globalBufferPool.acquire(input.length);
  output.set(input);
  softmaxInPlace(output, input.length);
  return output;
}

// ============================================================================
// Original Scaled Dot-Product Attention (backward compatible)
// ============================================================================

/**
 * Scaled Dot-Product Attention
 * The core attention mechanism
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
  const weight = expScore; // Simplified for single K,V pair

  // Weighted value
  const output = value.map(v => v * weight);

  return { output, weights: [weight] };
}

// ============================================================================
// Optimized Scaled Dot-Product Attention with TypedArrays
// ============================================================================

/**
 * Optimized scaled dot-product attention using TypedArrays and SIMD-style ops
 */
export function scaledDotProductAttentionOptimized(
  query: Float32Array,
  key: Float32Array,
  value: Float32Array,
  mask?: Float32Array | null
): { output: Float32Array; weights: Float32Array } {
  const dk = query.length;
  const scale = 1.0 / Math.sqrt(dk);

  // Compute attention score using 8x unrolled dot product
  let score = dotProduct8x(query, key, dk) * scale;

  // Apply mask if provided
  if (mask && mask[0] === 0) {
    score = -Infinity;
  }

  // Softmax (single score version)
  const expScore = Math.exp(score);
  const weight = expScore;

  // Weighted value (using buffer pool)
  const output = globalBufferPool.acquire(value.length);
  const unrolledLen = value.length - (value.length % 8);

  for (let i = 0; i < unrolledLen; i += 8) {
    output[i] = value[i] * weight;
    output[i + 1] = value[i + 1] * weight;
    output[i + 2] = value[i + 2] * weight;
    output[i + 3] = value[i + 3] * weight;
    output[i + 4] = value[i + 4] * weight;
    output[i + 5] = value[i + 5] * weight;
    output[i + 6] = value[i + 6] * weight;
    output[i + 7] = value[i + 7] * weight;
  }

  for (let i = unrolledLen; i < value.length; i++) {
    output[i] = value[i] * weight;
  }

  return { output, weights: new Float32Array([weight]) };
}

/**
 * Batch scaled dot-product attention for processing multiple queries at once
 * Processes multiple Q-K-V triplets efficiently
 */
export function batchScaledDotProductAttention(
  queries: Float32Array[],
  keys: Float32Array[],
  values: Float32Array[],
  masks?: (Float32Array | null)[]
): { outputs: Float32Array[]; weights: Float32Array[] } {
  const batchSize = queries.length;
  const outputs: Float32Array[] = [];
  const allWeights: Float32Array[] = [];

  // Process in batches of 4 for cache efficiency
  const batchOf4 = batchSize - (batchSize % 4);

  for (let b = 0; b < batchOf4; b += 4) {
    // Process 4 attention computations
    for (let i = 0; i < 4; i++) {
      const idx = b + i;
      const mask = masks ? masks[idx] : null;
      const { output, weights } = scaledDotProductAttentionOptimized(
        queries[idx],
        keys[idx],
        values[idx],
        mask
      );
      outputs.push(output);
      allWeights.push(weights);
    }
  }

  // Handle remainder
  for (let b = batchOf4; b < batchSize; b++) {
    const mask = masks ? masks[b] : null;
    const { output, weights } = scaledDotProductAttentionOptimized(
      queries[b],
      keys[b],
      values[b],
      mask
    );
    outputs.push(output);
    allWeights.push(weights);
  }

  return { outputs, weights: allWeights };
}

/**
 * Full attention computation over sequences (Q, K, V matrices)
 * Returns attention output and weights for all query positions
 */
export function batchSequenceAttention(
  queries: Float32Array,  // [seqLen * dim] flattened
  keys: Float32Array,     // [seqLen * dim] flattened
  values: Float32Array,   // [seqLen * dim] flattened
  seqLen: number,
  dim: number,
  mask?: Float32Array     // [seqLen * seqLen] attention mask
): { output: Float32Array; weights: Float32Array } {
  const scale = 1.0 / Math.sqrt(dim);
  const output = globalBufferPool.acquire(seqLen * dim);
  const weights = globalBufferPool.acquire(seqLen * seqLen);

  // Compute attention scores for each query position
  for (let qi = 0; qi < seqLen; qi++) {
    const qOffset = qi * dim;
    const scoresOffset = qi * seqLen;

    // Compute Q[qi] * K^T for all key positions
    for (let ki = 0; ki < seqLen; ki++) {
      const kOffset = ki * dim;
      let score = 0;

      // 8x unrolled dot product
      const unrolledDim = dim - (dim % 8);
      let sum0 = 0, sum1 = 0, sum2 = 0, sum3 = 0;
      let sum4 = 0, sum5 = 0, sum6 = 0, sum7 = 0;

      for (let d = 0; d < unrolledDim; d += 8) {
        sum0 += queries[qOffset + d] * keys[kOffset + d];
        sum1 += queries[qOffset + d + 1] * keys[kOffset + d + 1];
        sum2 += queries[qOffset + d + 2] * keys[kOffset + d + 2];
        sum3 += queries[qOffset + d + 3] * keys[kOffset + d + 3];
        sum4 += queries[qOffset + d + 4] * keys[kOffset + d + 4];
        sum5 += queries[qOffset + d + 5] * keys[kOffset + d + 5];
        sum6 += queries[qOffset + d + 6] * keys[kOffset + d + 6];
        sum7 += queries[qOffset + d + 7] * keys[kOffset + d + 7];
      }

      for (let d = unrolledDim; d < dim; d++) {
        sum0 += queries[qOffset + d] * keys[kOffset + d];
      }

      score = ((sum0 + sum1) + (sum2 + sum3) + (sum4 + sum5) + (sum6 + sum7)) * scale;

      // Apply mask if provided
      if (mask && mask[scoresOffset + ki] === 0) {
        score = -1e9; // Large negative instead of -Infinity for numerical stability
      }

      weights[scoresOffset + ki] = score;
    }

    // Softmax over the row
    softmaxInPlace(
      new Float32Array(weights.buffer, scoresOffset * 4, seqLen),
      seqLen
    );

    // Weighted sum of values
    const outOffset = qi * dim;
    for (let d = 0; d < dim; d++) {
      output[outOffset + d] = 0;
    }

    for (let vi = 0; vi < seqLen; vi++) {
      const vOffset = vi * dim;
      const w = weights[scoresOffset + vi];

      // 8x unrolled weighted add
      const unrolledDim = dim - (dim % 8);
      for (let d = 0; d < unrolledDim; d += 8) {
        output[outOffset + d] += w * values[vOffset + d];
        output[outOffset + d + 1] += w * values[vOffset + d + 1];
        output[outOffset + d + 2] += w * values[vOffset + d + 2];
        output[outOffset + d + 3] += w * values[vOffset + d + 3];
        output[outOffset + d + 4] += w * values[vOffset + d + 4];
        output[outOffset + d + 5] += w * values[vOffset + d + 5];
        output[outOffset + d + 6] += w * values[vOffset + d + 6];
        output[outOffset + d + 7] += w * values[vOffset + d + 7];
      }

      for (let d = unrolledDim; d < dim; d++) {
        output[outOffset + d] += w * values[vOffset + d];
      }
    }
  }

  return { output, weights };
}

/**
 * Multi-Head Attention (JavaScript fallback)
 *
 * Replaces broken @ruvector/attention.multiHeadAttention
 */
export class MultiHeadAttention {
  private numHeads: number;
  private hiddenDim: number;
  private headDim: number;
  private queryWeights: number[][][];
  private keyWeights: number[][][];
  private valueWeights: number[][][];
  private outputWeights: number[][];

  constructor(config: AttentionConfig) {
    // Validate config for security
    validateDimension(config.hiddenDim, 'hiddenDim');

    this.numHeads = Math.min(Math.max(1, config.numHeads || 8), 256);
    this.hiddenDim = config.hiddenDim;
    this.headDim = Math.floor(this.hiddenDim / this.numHeads);

    if (this.headDim < 1) {
      throw new Error('headDim must be at least 1 (hiddenDim / numHeads)');
    }

    // Initialize weights (random)
    this.queryWeights = this.initializeWeights();
    this.keyWeights = this.initializeWeights();
    this.valueWeights = this.initializeWeights();
    this.outputWeights = this.initializeOutputWeights();
  }

  private initializeWeights(): number[][][] {
    const weights: number[][][] = [];
    for (let h = 0; h < this.numHeads; h++) {
      const headWeights: number[][] = [];
      for (let i = 0; i < this.headDim; i++) {
        const row: number[] = [];
        for (let j = 0; j < this.hiddenDim; j++) {
          row.push((Math.random() - 0.5) * 0.1);
        }
        headWeights.push(row);
      }
      weights.push(headWeights);
    }
    return weights;
  }

  private initializeOutputWeights(): number[][] {
    const weights: number[][] = [];
    for (let i = 0; i < this.hiddenDim; i++) {
      const row: number[] = [];
      for (let j = 0; j < this.hiddenDim; j++) {
        row.push((Math.random() - 0.5) * 0.1);
      }
      weights.push(row);
    }
    return weights;
  }

  forward(
    query: number[],
    key: number[],
    value: number[],
    mask?: number[]
  ): { output: number[]; attentionWeights: number[][] } {
    const headOutputs: number[][] = [];
    const allWeights: number[][] = [];

    // Process each head
    for (let h = 0; h < this.numHeads; h++) {
      // Project to head dimension
      const q = this.project(query, this.queryWeights[h]);
      const k = this.project(key, this.keyWeights[h]);
      const v = this.project(value, this.valueWeights[h]);

      // Attention for this head
      const { output, weights } = scaledDotProductAttention(q, k, v, mask);

      headOutputs.push(output);
      allWeights.push(weights);
    }

    // Concatenate heads
    const concatenated = headOutputs.flat();

    // Output projection
    const output = this.project(concatenated, this.outputWeights);

    return { output, attentionWeights: allWeights };
  }

  private project(input: number[], weights: number[][]): number[] {
    const output: number[] = [];
    for (let i = 0; i < weights.length; i++) {
      let sum = 0;
      for (let j = 0; j < input.length; j++) {
        sum += input[j] * weights[i][j];
      }
      output.push(sum);
    }
    return output;
  }
}

// ============================================================================
// Optimized Multi-Head Attention with TypedArrays and SIMD-style operations
// ============================================================================

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
export class MultiHeadAttentionOptimized {
  private numHeads: number;
  private hiddenDim: number;
  private headDim: number;

  // Contiguous weight storage for cache efficiency
  // Shape: [numHeads * headDim * hiddenDim]
  private queryWeights: Float32Array;
  private keyWeights: Float32Array;
  private valueWeights: Float32Array;
  // Shape: [hiddenDim * hiddenDim]
  private outputWeights: Float32Array;

  // Pre-allocated buffers for intermediate results
  private headBuffer: Float32Array;
  private projBuffer: Float32Array;
  private concatBuffer: Float32Array;

  constructor(config: AttentionConfig) {
    // Validate config
    validateDimension(config.hiddenDim, 'hiddenDim');

    this.numHeads = Math.min(Math.max(1, config.numHeads || 8), 256);
    this.hiddenDim = config.hiddenDim;
    this.headDim = Math.floor(this.hiddenDim / this.numHeads);

    if (this.headDim < 1) {
      throw new Error('headDim must be at least 1 (hiddenDim / numHeads)');
    }

    const projSize = this.numHeads * this.headDim * this.hiddenDim;

    // Initialize contiguous weight arrays
    this.queryWeights = this.initializeProjectionWeights(projSize);
    this.keyWeights = this.initializeProjectionWeights(projSize);
    this.valueWeights = this.initializeProjectionWeights(projSize);
    this.outputWeights = this.initializeOutputWeights();

    // Pre-allocate buffers
    this.headBuffer = new Float32Array(this.headDim);
    this.projBuffer = new Float32Array(this.headDim * 3); // Q, K, V for one head
    this.concatBuffer = new Float32Array(this.numHeads * this.headDim);
  }

  private initializeProjectionWeights(size: number): Float32Array {
    const weights = new Float32Array(size);
    const scale = 0.1;
    for (let i = 0; i < size; i++) {
      weights[i] = (Math.random() - 0.5) * scale;
    }
    return weights;
  }

  private initializeOutputWeights(): Float32Array {
    const size = this.hiddenDim * this.hiddenDim;
    const weights = new Float32Array(size);
    const scale = 0.1;
    for (let i = 0; i < size; i++) {
      weights[i] = (Math.random() - 0.5) * scale;
    }
    return weights;
  }

  /**
   * Forward pass with optimized TypedArray operations
   */
  forward(
    query: Float32Array,
    key: Float32Array,
    value: Float32Array,
    mask?: Float32Array | null
  ): { output: Float32Array; attentionWeights: Float32Array[] } {
    const allWeights: Float32Array[] = [];
    const headOutputs: Float32Array[] = [];

    // Process each head with optimized projections
    for (let h = 0; h < this.numHeads; h++) {
      const weightOffset = h * this.headDim * this.hiddenDim;

      // Project Q, K, V for this head using 8x unrolled matmul
      const q = this.projectOptimized(query, this.queryWeights, weightOffset);
      const k = this.projectOptimized(key, this.keyWeights, weightOffset);
      const v = this.projectOptimized(value, this.valueWeights, weightOffset);

      // Scaled dot-product attention
      const { output, weights } = scaledDotProductAttentionOptimized(q, k, v, mask);

      headOutputs.push(output);
      allWeights.push(weights);
    }

    // Concatenate heads into pre-allocated buffer
    let offset = 0;
    for (let h = 0; h < this.numHeads; h++) {
      this.concatBuffer.set(headOutputs[h], offset);
      offset += headOutputs[h].length;
    }

    // Output projection
    const output = globalBufferPool.acquire(this.hiddenDim);
    matVecMul8x(
      this.outputWeights,
      this.concatBuffer,
      output,
      this.hiddenDim,
      this.hiddenDim
    );

    return { output, attentionWeights: allWeights };
  }

  /**
   * Batch forward pass for multiple inputs
   */
  batchForward(
    queries: Float32Array[],
    keys: Float32Array[],
    values: Float32Array[],
    masks?: (Float32Array | null)[]
  ): { outputs: Float32Array[]; attentionWeights: Float32Array[][] } {
    const outputs: Float32Array[] = [];
    const allWeights: Float32Array[][] = [];

    for (let i = 0; i < queries.length; i++) {
      const mask = masks ? masks[i] : null;
      const { output, attentionWeights } = this.forward(
        queries[i],
        keys[i],
        values[i],
        mask
      );
      outputs.push(output);
      allWeights.push(attentionWeights);
    }

    return { outputs, attentionWeights: allWeights };
  }

  /**
   * Optimized projection using contiguous weight storage and 8x unrolling
   */
  private projectOptimized(
    input: Float32Array,
    weights: Float32Array,
    weightOffset: number
  ): Float32Array {
    const output = globalBufferPool.acquire(this.headDim);
    const inputLen = input.length;
    const unrolledLen = inputLen - (inputLen % 8);

    for (let i = 0; i < this.headDim; i++) {
      let sum0 = 0, sum1 = 0, sum2 = 0, sum3 = 0;
      let sum4 = 0, sum5 = 0, sum6 = 0, sum7 = 0;

      const rowOffset = weightOffset + i * inputLen;

      // 8x unrolled inner loop
      for (let j = 0; j < unrolledLen; j += 8) {
        sum0 += input[j] * weights[rowOffset + j];
        sum1 += input[j + 1] * weights[rowOffset + j + 1];
        sum2 += input[j + 2] * weights[rowOffset + j + 2];
        sum3 += input[j + 3] * weights[rowOffset + j + 3];
        sum4 += input[j + 4] * weights[rowOffset + j + 4];
        sum5 += input[j + 5] * weights[rowOffset + j + 5];
        sum6 += input[j + 6] * weights[rowOffset + j + 6];
        sum7 += input[j + 7] * weights[rowOffset + j + 7];
      }

      // Handle remainder
      for (let j = unrolledLen; j < inputLen; j++) {
        sum0 += input[j] * weights[rowOffset + j];
      }

      output[i] = (sum0 + sum1) + (sum2 + sum3) + (sum4 + sum5) + (sum6 + sum7);
    }

    return output;
  }

  /**
   * Release buffers back to pool (call when done with outputs)
   */
  releaseBuffer(buffer: Float32Array): void {
    globalBufferPool.release(buffer);
  }

  /**
   * Get weight matrices (for serialization/loading)
   */
  getWeights(): {
    query: Float32Array;
    key: Float32Array;
    value: Float32Array;
    output: Float32Array;
  } {
    return {
      query: this.queryWeights,
      key: this.keyWeights,
      value: this.valueWeights,
      output: this.outputWeights,
    };
  }

  /**
   * Set weight matrices (for loading pre-trained weights)
   */
  setWeights(weights: {
    query: Float32Array;
    key: Float32Array;
    value: Float32Array;
    output: Float32Array;
  }): void {
    this.queryWeights.set(weights.query);
    this.keyWeights.set(weights.key);
    this.valueWeights.set(weights.value);
    this.outputWeights.set(weights.output);
  }
}

/**
 * Flash Attention (optimized fallback)
 *
 * Replaces broken @ruvector/attention.flashAttention
 * Uses tiling/chunking for better memory efficiency
 */
export class FlashAttention {
  private hiddenDim: number;
  private blockSize: number;

  constructor(config: AttentionConfig) {
    this.hiddenDim = config.hiddenDim;
    this.blockSize = Math.min(64, this.hiddenDim); // Tile size
  }

  forward(
    query: number[][],
    key: number[][],
    value: number[][],
    numHeads: number = 8
  ): { output: number[][]; attentionScores: number[][] } {
    const seqLen = query.length;
    const headDim = this.hiddenDim / numHeads;

    const output: number[][] = [];
    const attentionScores: number[][] = [];

    // Process in blocks for memory efficiency
    for (let i = 0; i < seqLen; i += this.blockSize) {
      const blockEnd = Math.min(i + this.blockSize, seqLen);

      for (let qi = i; qi < blockEnd; qi++) {
        const scores: number[] = [];
        let maxScore = -Infinity;

        // Compute attention scores for this query
        for (let ki = 0; ki < seqLen; ki++) {
          let score = 0;
          for (let d = 0; d < query[qi].length; d++) {
            score += query[qi][d] * key[ki][d];
          }
          score /= Math.sqrt(headDim);
          scores.push(score);
          maxScore = Math.max(maxScore, score);
        }

        // Numerically stable softmax
        const expScores = scores.map(s => Math.exp(s - maxScore));
        const sumExp = expScores.reduce((a, b) => a + b, 0);
        const weights = expScores.map(e => e / sumExp);

        // Weighted sum of values
        const outputRow = new Array(value[0].length).fill(0);
        for (let vi = 0; vi < seqLen; vi++) {
          for (let d = 0; d < value[vi].length; d++) {
            outputRow[d] += weights[vi] * value[vi][d];
          }
        }

        output.push(outputRow);
        attentionScores.push(weights);
      }
    }

    return { output, attentionScores };
  }
}

// ============================================================================
// Optimized Flash Attention with improved tiling and TypedArrays
// ============================================================================

/**
 * Flash Attention Configuration
 */
export interface FlashAttentionConfig extends AttentionConfig {
  blockSizeQ?: number;  // Query block size
  blockSizeKV?: number; // Key/Value block size
  causal?: boolean;     // Use causal masking
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
export class FlashAttentionOptimized {
  private hiddenDim: number;
  private blockSizeQ: number;
  private blockSizeKV: number;
  private causal: boolean;

  // Pre-allocated buffers
  private scoresBuffer: Float32Array;
  private expBuffer: Float32Array;
  private outputBuffer: Float32Array;
  private maxBuffer: Float32Array;
  private sumBuffer: Float32Array;

  constructor(config: FlashAttentionConfig) {
    // Validate config
    validateDimension(config.hiddenDim, 'hiddenDim');

    this.hiddenDim = config.hiddenDim;

    // Optimal block sizes for L1/L2 cache (typically 32KB-256KB)
    // Bounded for security
    this.blockSizeQ = Math.min(Math.max(8, config.blockSizeQ || 32), 256);
    this.blockSizeKV = Math.min(Math.max(8, config.blockSizeKV || 64), 256);
    this.causal = config.causal || false;

    // Validate that block sizes don't cause excessive memory allocation
    const maxBlockMemory = this.blockSizeQ * this.blockSizeKV * 4 * 2; // scores + exp buffers
    const maxOutputMemory = this.blockSizeQ * this.hiddenDim * 4;
    if (maxBlockMemory + maxOutputMemory > 64 * 1024 * 1024) { // 64MB limit
      throw new Error('Block sizes would require excessive memory allocation');
    }

    // Pre-allocate buffers for maximum expected sizes
    this.scoresBuffer = new Float32Array(this.blockSizeQ * this.blockSizeKV);
    this.expBuffer = new Float32Array(this.blockSizeQ * this.blockSizeKV);
    this.outputBuffer = new Float32Array(this.blockSizeQ * this.hiddenDim);
    this.maxBuffer = new Float32Array(this.blockSizeQ);
    this.sumBuffer = new Float32Array(this.blockSizeQ);
  }

  /**
   * Forward pass with optimized tiling strategy
   * Uses online softmax for memory efficiency
   */
  forward(
    query: Float32Array,   // [seqLen * dim] flattened
    key: Float32Array,     // [seqLen * dim] flattened
    value: Float32Array,   // [seqLen * dim] flattened
    seqLen: number,
    dim: number,
    numHeads: number = 8
  ): { output: Float32Array; attentionScores: Float32Array } {
    // Validate inputs
    validateFloat32Array(query, 'query');
    validateFloat32Array(key, 'key');
    validateFloat32Array(value, 'value');
    validateSeqLength(seqLen, 'seqLen');
    validateDimension(dim, 'dim');

    // Validate array sizes match expected dimensions
    const expectedSize = seqLen * dim;
    if (query.length < expectedSize || key.length < expectedSize || value.length < expectedSize) {
      throw new Error(`Input arrays too small for seqLen=${seqLen}, dim=${dim}`);
    }

    const headDim = dim / numHeads;
    if (headDim < 1) {
      throw new Error('Invalid numHeads: results in headDim < 1');
    }
    const scale = 1.0 / Math.sqrt(headDim);

    // Allocate output arrays
    const output = globalBufferPool.acquire(seqLen * dim);
    const attentionScores = globalBufferPool.acquire(seqLen * seqLen);

    // Initialize output to zero
    output.fill(0);

    // Process query blocks
    for (let qBlockStart = 0; qBlockStart < seqLen; qBlockStart += this.blockSizeQ) {
      const qBlockEnd = Math.min(qBlockStart + this.blockSizeQ, seqLen);
      const qBlockSize = qBlockEnd - qBlockStart;

      // Initialize running max and sum for online softmax
      this.maxBuffer.fill(-Infinity);
      this.sumBuffer.fill(0);
      this.outputBuffer.fill(0);

      // Process key-value blocks
      const kvEnd = this.causal ? qBlockEnd : seqLen;

      for (let kvBlockStart = 0; kvBlockStart < kvEnd; kvBlockStart += this.blockSizeKV) {
        const kvBlockEnd = Math.min(kvBlockStart + this.blockSizeKV, kvEnd);
        const kvBlockSize = kvBlockEnd - kvBlockStart;

        // Compute attention scores for this block (Q_block @ K_block^T)
        this.computeBlockScores(
          query, key, qBlockStart, qBlockEnd, kvBlockStart, kvBlockEnd,
          dim, scale
        );

        // Apply causal mask if needed
        if (this.causal) {
          for (let qi = 0; qi < qBlockSize; qi++) {
            const globalQi = qBlockStart + qi;
            for (let ki = 0; ki < kvBlockSize; ki++) {
              const globalKi = kvBlockStart + ki;
              if (globalKi > globalQi) {
                this.scoresBuffer[qi * kvBlockSize + ki] = -1e9;
              }
            }
          }
        }

        // Online softmax update and output accumulation
        this.updateOnlineSoftmax(
          value, qBlockSize, kvBlockSize, kvBlockStart, dim
        );

        // Store attention scores
        for (let qi = 0; qi < qBlockSize; qi++) {
          const globalQi = qBlockStart + qi;
          for (let ki = 0; ki < kvBlockSize; ki++) {
            const globalKi = kvBlockStart + ki;
            attentionScores[globalQi * seqLen + globalKi] = this.scoresBuffer[qi * kvBlockSize + ki];
          }
        }
      }

      // Normalize output by sum
      for (let qi = 0; qi < qBlockSize; qi++) {
        const globalQi = qBlockStart + qi;
        const invSum = 1.0 / (this.sumBuffer[qi] + 1e-8);

        const outOffset = globalQi * dim;
        const localOffset = qi * dim;

        // 8x unrolled normalization
        const unrolledDim = dim - (dim % 8);
        for (let d = 0; d < unrolledDim; d += 8) {
          output[outOffset + d] = this.outputBuffer[localOffset + d] * invSum;
          output[outOffset + d + 1] = this.outputBuffer[localOffset + d + 1] * invSum;
          output[outOffset + d + 2] = this.outputBuffer[localOffset + d + 2] * invSum;
          output[outOffset + d + 3] = this.outputBuffer[localOffset + d + 3] * invSum;
          output[outOffset + d + 4] = this.outputBuffer[localOffset + d + 4] * invSum;
          output[outOffset + d + 5] = this.outputBuffer[localOffset + d + 5] * invSum;
          output[outOffset + d + 6] = this.outputBuffer[localOffset + d + 6] * invSum;
          output[outOffset + d + 7] = this.outputBuffer[localOffset + d + 7] * invSum;
        }

        for (let d = unrolledDim; d < dim; d++) {
          output[outOffset + d] = this.outputBuffer[localOffset + d] * invSum;
        }
      }
    }

    return { output, attentionScores };
  }

  /**
   * Compute attention scores for a block pair with 8x unrolling
   */
  private computeBlockScores(
    query: Float32Array,
    key: Float32Array,
    qStart: number,
    qEnd: number,
    kStart: number,
    kEnd: number,
    dim: number,
    scale: number
  ): void {
    const qBlockSize = qEnd - qStart;
    const kBlockSize = kEnd - kStart;
    const unrolledDim = dim - (dim % 8);

    for (let qi = 0; qi < qBlockSize; qi++) {
      const qOffset = (qStart + qi) * dim;

      for (let ki = 0; ki < kBlockSize; ki++) {
        const kOffset = (kStart + ki) * dim;

        // 8x unrolled dot product
        let sum0 = 0, sum1 = 0, sum2 = 0, sum3 = 0;
        let sum4 = 0, sum5 = 0, sum6 = 0, sum7 = 0;

        for (let d = 0; d < unrolledDim; d += 8) {
          sum0 += query[qOffset + d] * key[kOffset + d];
          sum1 += query[qOffset + d + 1] * key[kOffset + d + 1];
          sum2 += query[qOffset + d + 2] * key[kOffset + d + 2];
          sum3 += query[qOffset + d + 3] * key[kOffset + d + 3];
          sum4 += query[qOffset + d + 4] * key[kOffset + d + 4];
          sum5 += query[qOffset + d + 5] * key[kOffset + d + 5];
          sum6 += query[qOffset + d + 6] * key[kOffset + d + 6];
          sum7 += query[qOffset + d + 7] * key[kOffset + d + 7];
        }

        for (let d = unrolledDim; d < dim; d++) {
          sum0 += query[qOffset + d] * key[kOffset + d];
        }

        const score = ((sum0 + sum1) + (sum2 + sum3) + (sum4 + sum5) + (sum6 + sum7)) * scale;
        this.scoresBuffer[qi * kBlockSize + ki] = score;
      }
    }
  }

  /**
   * Online softmax update with output accumulation
   * This is the core of flash attention's memory efficiency
   */
  private updateOnlineSoftmax(
    value: Float32Array,
    qBlockSize: number,
    kvBlockSize: number,
    kvBlockStart: number,
    dim: number
  ): void {
    for (let qi = 0; qi < qBlockSize; qi++) {
      // Find new max
      let newMax = this.maxBuffer[qi];
      for (let ki = 0; ki < kvBlockSize; ki++) {
        const score = this.scoresBuffer[qi * kvBlockSize + ki];
        if (score > newMax) newMax = score;
      }

      // Compute scaling factor for existing values
      const oldMax = this.maxBuffer[qi];
      const rescale = Math.exp(oldMax - newMax);

      // Update running sum with rescaling
      let newSum = this.sumBuffer[qi] * rescale;

      // Compute exp scores and accumulate
      const localOffset = qi * dim;

      for (let ki = 0; ki < kvBlockSize; ki++) {
        const expScore = Math.exp(this.scoresBuffer[qi * kvBlockSize + ki] - newMax);
        this.expBuffer[qi * kvBlockSize + ki] = expScore;
        newSum += expScore;
      }

      // Rescale existing output
      if (rescale !== 1.0) {
        const unrolledDim = dim - (dim % 8);
        for (let d = 0; d < unrolledDim; d += 8) {
          this.outputBuffer[localOffset + d] *= rescale;
          this.outputBuffer[localOffset + d + 1] *= rescale;
          this.outputBuffer[localOffset + d + 2] *= rescale;
          this.outputBuffer[localOffset + d + 3] *= rescale;
          this.outputBuffer[localOffset + d + 4] *= rescale;
          this.outputBuffer[localOffset + d + 5] *= rescale;
          this.outputBuffer[localOffset + d + 6] *= rescale;
          this.outputBuffer[localOffset + d + 7] *= rescale;
        }
        for (let d = unrolledDim; d < dim; d++) {
          this.outputBuffer[localOffset + d] *= rescale;
        }
      }

      // Add new weighted values
      for (let ki = 0; ki < kvBlockSize; ki++) {
        const weight = this.expBuffer[qi * kvBlockSize + ki];
        const vOffset = (kvBlockStart + ki) * dim;

        // 8x unrolled weighted add
        const unrolledDim = dim - (dim % 8);
        for (let d = 0; d < unrolledDim; d += 8) {
          this.outputBuffer[localOffset + d] += weight * value[vOffset + d];
          this.outputBuffer[localOffset + d + 1] += weight * value[vOffset + d + 1];
          this.outputBuffer[localOffset + d + 2] += weight * value[vOffset + d + 2];
          this.outputBuffer[localOffset + d + 3] += weight * value[vOffset + d + 3];
          this.outputBuffer[localOffset + d + 4] += weight * value[vOffset + d + 4];
          this.outputBuffer[localOffset + d + 5] += weight * value[vOffset + d + 5];
          this.outputBuffer[localOffset + d + 6] += weight * value[vOffset + d + 6];
          this.outputBuffer[localOffset + d + 7] += weight * value[vOffset + d + 7];
        }
        for (let d = unrolledDim; d < dim; d++) {
          this.outputBuffer[localOffset + d] += weight * value[vOffset + d];
        }
      }

      // Update max and sum
      this.maxBuffer[qi] = newMax;
      this.sumBuffer[qi] = newSum;
    }
  }

  /**
   * Batch forward for processing multiple sequences
   */
  batchForward(
    queries: Float32Array[],
    keys: Float32Array[],
    values: Float32Array[],
    seqLens: number[],
    dim: number,
    numHeads: number = 8
  ): { outputs: Float32Array[]; attentionScores: Float32Array[] } {
    const outputs: Float32Array[] = [];
    const allScores: Float32Array[] = [];

    for (let i = 0; i < queries.length; i++) {
      const { output, attentionScores } = this.forward(
        queries[i],
        keys[i],
        values[i],
        seqLens[i],
        dim,
        numHeads
      );
      outputs.push(output);
      allScores.push(attentionScores);
    }

    return { outputs, attentionScores: allScores };
  }

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
  backward(
    dOutput: Float32Array,
    query: Float32Array,
    key: Float32Array,
    value: Float32Array,
    attentionScores: Float32Array,
    seqLen: number,
    dim: number
  ): { dQuery: Float32Array; dKey: Float32Array; dValue: Float32Array } {
    // Validate inputs
    validateFloat32Array(dOutput, 'dOutput');
    validateFloat32Array(query, 'query');
    validateFloat32Array(key, 'key');
    validateFloat32Array(value, 'value');
    validateFloat32Array(attentionScores, 'attentionScores');
    validateSeqLength(seqLen, 'seqLen');
    validateDimension(dim, 'dim');

    const dQuery = globalBufferPool.acquire(seqLen * dim);
    const dKey = globalBufferPool.acquire(seqLen * dim);
    const dValue = globalBufferPool.acquire(seqLen * dim);

    // Initialize gradients to zero
    dQuery.fill(0);
    dKey.fill(0);
    dValue.fill(0);

    // Scaling factor (must match forward pass)
    const numHeads = 8; // Default from forward
    const headDim = dim / numHeads;
    const scale = 1.0 / Math.sqrt(headDim);

    // First: Normalize attention scores to get attention weights A
    // attentionScores contains the raw scores S, we need to apply softmax
    // In the forward pass, scores were stored after softmax, so use them directly
    // But the scores buffer may contain raw scores - we recompute softmax for numerical stability

    // Allocate temporary buffer for attention weights (normalized)
    const attentionWeights = globalBufferPool.acquire(seqLen * seqLen);

    // Apply softmax row-wise to get attention weights A
    for (let i = 0; i < seqLen; i++) {
      const rowOffset = i * seqLen;

      // Find max for numerical stability
      let maxScore = -Infinity;
      for (let j = 0; j < seqLen; j++) {
        const score = attentionScores[rowOffset + j];
        if (score > maxScore) maxScore = score;
      }

      // Compute exp and sum
      let sumExp = 0;
      for (let j = 0; j < seqLen; j++) {
        const expVal = Math.exp(attentionScores[rowOffset + j] - maxScore);
        attentionWeights[rowOffset + j] = expVal;
        sumExp += expVal;
      }

      // Normalize
      const invSum = 1.0 / (sumExp + EPSILON);
      for (let j = 0; j < seqLen; j++) {
        attentionWeights[rowOffset + j] *= invSum;
      }
    }

    // Step 1: Compute dV = A^T @ dO
    // dV[j, d] = sum_i A[i, j] * dO[i, d]
    for (let j = 0; j < seqLen; j++) {
      const dVOffset = j * dim;

      for (let i = 0; i < seqLen; i++) {
        const aWeight = attentionWeights[i * seqLen + j]; // A^T[j, i] = A[i, j]
        const dOOffset = i * dim;

        // 8x unrolled accumulation
        const unrolledDim = dim - (dim % 8);
        for (let d = 0; d < unrolledDim; d += 8) {
          dValue[dVOffset + d] += aWeight * dOutput[dOOffset + d];
          dValue[dVOffset + d + 1] += aWeight * dOutput[dOOffset + d + 1];
          dValue[dVOffset + d + 2] += aWeight * dOutput[dOOffset + d + 2];
          dValue[dVOffset + d + 3] += aWeight * dOutput[dOOffset + d + 3];
          dValue[dVOffset + d + 4] += aWeight * dOutput[dOOffset + d + 4];
          dValue[dVOffset + d + 5] += aWeight * dOutput[dOOffset + d + 5];
          dValue[dVOffset + d + 6] += aWeight * dOutput[dOOffset + d + 6];
          dValue[dVOffset + d + 7] += aWeight * dOutput[dOOffset + d + 7];
        }
        for (let d = unrolledDim; d < dim; d++) {
          dValue[dVOffset + d] += aWeight * dOutput[dOOffset + d];
        }
      }
    }

    // Step 2: Compute dA = dO @ V^T
    // dA[i, j] = sum_d dO[i, d] * V[j, d]
    const dAttention = globalBufferPool.acquire(seqLen * seqLen);
    dAttention.fill(0);

    for (let i = 0; i < seqLen; i++) {
      const dOOffset = i * dim;
      const dARowOffset = i * seqLen;

      for (let j = 0; j < seqLen; j++) {
        const vOffset = j * dim;
        let dotProduct = 0;

        // 8x unrolled dot product
        const unrolledDim = dim - (dim % 8);
        let sum0 = 0, sum1 = 0, sum2 = 0, sum3 = 0;
        let sum4 = 0, sum5 = 0, sum6 = 0, sum7 = 0;

        for (let d = 0; d < unrolledDim; d += 8) {
          sum0 += dOutput[dOOffset + d] * value[vOffset + d];
          sum1 += dOutput[dOOffset + d + 1] * value[vOffset + d + 1];
          sum2 += dOutput[dOOffset + d + 2] * value[vOffset + d + 2];
          sum3 += dOutput[dOOffset + d + 3] * value[vOffset + d + 3];
          sum4 += dOutput[dOOffset + d + 4] * value[vOffset + d + 4];
          sum5 += dOutput[dOOffset + d + 5] * value[vOffset + d + 5];
          sum6 += dOutput[dOOffset + d + 6] * value[vOffset + d + 6];
          sum7 += dOutput[dOOffset + d + 7] * value[vOffset + d + 7];
        }
        dotProduct = (sum0 + sum1) + (sum2 + sum3) + (sum4 + sum5) + (sum6 + sum7);

        for (let d = unrolledDim; d < dim; d++) {
          dotProduct += dOutput[dOOffset + d] * value[vOffset + d];
        }

        dAttention[dARowOffset + j] = dotProduct;
      }
    }

    // Step 3: Compute dS = softmax_backward(dA, A)
    // For softmax: dS_ij = A_ij * (dA_ij - sum_k(A_ik * dA_ik))
    const dScores = globalBufferPool.acquire(seqLen * seqLen);

    for (let i = 0; i < seqLen; i++) {
      const rowOffset = i * seqLen;

      // Compute sum_k(A_ik * dA_ik) for this row
      let sumAdA = 0;
      for (let k = 0; k < seqLen; k++) {
        sumAdA += attentionWeights[rowOffset + k] * dAttention[rowOffset + k];
      }

      // Compute dS_ij = A_ij * (dA_ij - sumAdA) * scale
      for (let j = 0; j < seqLen; j++) {
        const idx = rowOffset + j;
        dScores[idx] = attentionWeights[idx] * (dAttention[idx] - sumAdA) * scale;
      }
    }

    // Step 4: Compute dQ = dS @ K
    // dQ[i, d] = sum_j dS[i, j] * K[j, d]
    for (let i = 0; i < seqLen; i++) {
      const dQOffset = i * dim;
      const dSRowOffset = i * seqLen;

      for (let j = 0; j < seqLen; j++) {
        const dSValue = dScores[dSRowOffset + j];
        const kOffset = j * dim;

        // 8x unrolled accumulation
        const unrolledDim = dim - (dim % 8);
        for (let d = 0; d < unrolledDim; d += 8) {
          dQuery[dQOffset + d] += dSValue * key[kOffset + d];
          dQuery[dQOffset + d + 1] += dSValue * key[kOffset + d + 1];
          dQuery[dQOffset + d + 2] += dSValue * key[kOffset + d + 2];
          dQuery[dQOffset + d + 3] += dSValue * key[kOffset + d + 3];
          dQuery[dQOffset + d + 4] += dSValue * key[kOffset + d + 4];
          dQuery[dQOffset + d + 5] += dSValue * key[kOffset + d + 5];
          dQuery[dQOffset + d + 6] += dSValue * key[kOffset + d + 6];
          dQuery[dQOffset + d + 7] += dSValue * key[kOffset + d + 7];
        }
        for (let d = unrolledDim; d < dim; d++) {
          dQuery[dQOffset + d] += dSValue * key[kOffset + d];
        }
      }
    }

    // Step 5: Compute dK = dS^T @ Q
    // dK[j, d] = sum_i dS[i, j] * Q[i, d]
    for (let j = 0; j < seqLen; j++) {
      const dKOffset = j * dim;

      for (let i = 0; i < seqLen; i++) {
        const dSValue = dScores[i * seqLen + j]; // dS^T[j, i] = dS[i, j]
        const qOffset = i * dim;

        // 8x unrolled accumulation
        const unrolledDim = dim - (dim % 8);
        for (let d = 0; d < unrolledDim; d += 8) {
          dKey[dKOffset + d] += dSValue * query[qOffset + d];
          dKey[dKOffset + d + 1] += dSValue * query[qOffset + d + 1];
          dKey[dKOffset + d + 2] += dSValue * query[qOffset + d + 2];
          dKey[dKOffset + d + 3] += dSValue * query[qOffset + d + 3];
          dKey[dKOffset + d + 4] += dSValue * query[qOffset + d + 4];
          dKey[dKOffset + d + 5] += dSValue * query[qOffset + d + 5];
          dKey[dKOffset + d + 6] += dSValue * query[qOffset + d + 6];
          dKey[dKOffset + d + 7] += dSValue * query[qOffset + d + 7];
        }
        for (let d = unrolledDim; d < dim; d++) {
          dKey[dKOffset + d] += dSValue * query[qOffset + d];
        }
      }
    }

    // Release temporary buffers
    globalBufferPool.release(attentionWeights);
    globalBufferPool.release(dAttention);
    globalBufferPool.release(dScores);

    return { dQuery, dKey, dValue };
  }

  /**
   * Release buffers back to pool
   */
  releaseBuffer(buffer: Float32Array): void {
    globalBufferPool.release(buffer);
  }
}

// ============================================================================
// Batch Multi-Head Attention
// ============================================================================

/**
 * Batch multi-head attention for processing multiple queries efficiently
 * Combines optimized MHA with batch processing
 */
export function batchMultiHeadAttention(
  attention: MultiHeadAttentionOptimized,
  queries: Float32Array[],
  keys: Float32Array[],
  values: Float32Array[],
  masks?: (Float32Array | null)[]
): { outputs: Float32Array[]; attentionWeights: Float32Array[][] } {
  return attention.batchForward(queries, keys, values, masks);
}

/**
 * Linear Attention (fallback)
 *
 * O(n) complexity approximation of attention
 */
export class LinearAttention {
  private hiddenDim: number;
  private featureMap: (x: number) => number;

  constructor(config: AttentionConfig) {
    this.hiddenDim = config.hiddenDim;
    // ELU feature map
    this.featureMap = (x: number) => (x > 0 ? x : Math.exp(x) - 1);
  }

  forward(
    query: number[][],
    key: number[][],
    value: number[][]
  ): { output: number[][] } {
    const seqLen = query.length;
    const dim = value[0].length;

    // Apply feature map
    const queryMapped = query.map(q => q.map(this.featureMap));
    const keyMapped = key.map(k => k.map(this.featureMap));

    // Compute K^T V (dimension: [dim, valueDim])
    const ktv: number[][] = Array.from({ length: this.hiddenDim }, () =>
      Array(dim).fill(0)
    );

    for (let i = 0; i < seqLen; i++) {
      for (let d1 = 0; d1 < this.hiddenDim; d1++) {
        for (let d2 = 0; d2 < dim; d2++) {
          ktv[d1][d2] += keyMapped[i][d1] * value[i][d2];
        }
      }
    }

    // Compute Q (K^T V)
    const output: number[][] = [];
    for (let i = 0; i < seqLen; i++) {
      const row: number[] = [];
      for (let d2 = 0; d2 < dim; d2++) {
        let sum = 0;
        for (let d1 = 0; d1 < this.hiddenDim; d1++) {
          sum += queryMapped[i][d1] * ktv[d1][d2];
        }
        row.push(sum);
      }

      // Normalize
      const normSum = queryMapped[i].reduce((a, b) => a + b, 0);
      output.push(row.map(v => v / (normSum + 1e-8)));
    }

    return { output };
  }
}

/**
 * Hyperbolic Attention (simplified fallback)
 *
 * Approximation using hyperbolic geometry
 */
export class HyperbolicAttention {
  private hiddenDim: number;
  private curvature: number;

  constructor(config: AttentionConfig) {
    this.hiddenDim = config.hiddenDim;
    this.curvature = -1.0; // Poincaré ball curvature
  }

  forward(
    query: number[],
    key: number[],
    value: number[]
  ): { output: number[]; distance: number } {
    // Hyperbolic distance (simplified)
    const distance = this.hyperbolicDistance(query, key);

    // Attention weight based on hyperbolic distance
    const weight = Math.exp(-distance);

    // Weighted value
    const output = value.map(v => v * weight);

    return { output, distance };
  }

  private hyperbolicDistance(a: number[], b: number[]): number {
    // Simplified hyperbolic distance in Poincare ball
    let normDiffSq = 0;
    for (let i = 0; i < a.length; i++) {
      const diff = a[i] - b[i];
      normDiffSq += diff * diff;
    }

    const normASq = a.reduce((sum, v) => sum + v * v, 0);
    const normBSq = b.reduce((sum, v) => sum + v * v, 0);

    const numerator = normDiffSq;
    const denominator = (1 - normASq) * (1 - normBSq);

    // Guard against division by zero and negative values
    if (denominator <= EPSILON) {
      return Infinity; // Points are on or outside the boundary
    }

    const acoshArg = 1 + (2 * numerator) / denominator;

    // Guard against invalid acosh argument (must be >= 1)
    if (!Number.isFinite(acoshArg) || acoshArg < 1) {
      return 0; // Return 0 for identical points or numerical issues
    }

    return Math.acosh(acoshArg);
  }
}

/**
 * MoE (Mixture of Experts) Attention (fallback)
 *
 * Routes to different expert attention modules
 */
export class MoEAttention {
  private experts: MultiHeadAttention[];
  private numExperts: number;
  private gatingWeights: number[][];

  constructor(config: AttentionConfig & { numExperts?: number }) {
    this.numExperts = config.numExperts || 4;
    this.experts = Array.from(
      { length: this.numExperts },
      () => new MultiHeadAttention(config)
    );

    // Initialize gating network weights
    this.gatingWeights = Array.from({ length: this.numExperts }, () =>
      Array.from({ length: config.hiddenDim }, () => (Math.random() - 0.5) * 0.1)
    );
  }

  forward(
    query: number[],
    key: number[],
    value: number[],
    topK: number = 2
  ): { output: number[]; expertWeights: number[] } {
    // Compute gating scores
    const gatingScores = this.gatingWeights.map(weights => {
      let score = 0;
      for (let i = 0; i < query.length; i++) {
        score += query[i] * weights[i];
      }
      return score;
    });

    // Softmax over top-K experts
    const expScores = gatingScores.map(s => Math.exp(s));
    const sumExp = expScores.reduce((a, b) => a + b, 0);
    const expertWeights = expScores.map(e => e / sumExp);

    // Get top-K experts
    const expertIndices = expertWeights
      .map((weight, idx) => ({ weight, idx }))
      .sort((a, b) => b.weight - a.weight)
      .slice(0, topK);

    // Weighted combination of expert outputs
    const output = new Array(query.length).fill(0);

    for (const { weight, idx } of expertIndices) {
      const expertOutput = this.experts[idx].forward(query, key, value).output;
      for (let i = 0; i < output.length; i++) {
        output[i] += weight * expertOutput[i];
      }
    }

    return { output, expertWeights };
  }
}

/**
 * Check if native attention is available
 */
export function isNativeAttentionAvailable(): boolean {
  try {
    const attention = require('@ruvector/attention');
    // Try a simple operation
    const result = attention.flashAttention(
      new Float32Array([1, 0]),
      new Float32Array([1, 0]),
      new Float32Array([1, 0]),
      1
    );
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

/**
 * Factory function to create optimized attention modules
 */
export function createAttentionOptimized(
  type: 'multi-head' | 'flash',
  config: AttentionConfig | FlashAttentionConfig
): MultiHeadAttentionOptimized | FlashAttentionOptimized {
  switch (type) {
    case 'multi-head':
      return new MultiHeadAttentionOptimized(config);
    case 'flash':
      return new FlashAttentionOptimized(config as FlashAttentionConfig);
    default:
      return new MultiHeadAttentionOptimized(config);
  }
}

// ============================================================================
// Utility Functions for TypedArray Conversion
// ============================================================================

/**
 * Convert number[] to Float32Array
 */
export function toFloat32Array(arr: number[]): Float32Array {
  return new Float32Array(arr);
}

/**
 * Convert Float32Array to number[]
 */
export function toNumberArray(arr: Float32Array): number[] {
  return Array.from(arr);
}

/**
 * Convert 2D number[][] to flattened Float32Array
 */
export function flatten2D(arr: number[][]): Float32Array {
  const totalLen = arr.reduce((sum, row) => sum + row.length, 0);
  const result = new Float32Array(totalLen);
  let offset = 0;
  for (const row of arr) {
    result.set(row, offset);
    offset += row.length;
  }
  return result;
}

/**
 * Unflatten Float32Array to 2D array
 */
export function unflatten2D(arr: Float32Array, rows: number, cols: number): number[][] {
  const result: number[][] = [];
  for (let i = 0; i < rows; i++) {
    result.push(Array.from(arr.slice(i * cols, (i + 1) * cols)));
  }
  return result;
}

// ============================================================================
// Buffer Pool Export for External Memory Management
// ============================================================================

/**
 * Get the global buffer pool for external memory management
 */
export function getBufferPool(): {
  acquire: (size: number) => Float32Array;
  release: (buffer: Float32Array) => void;
  clear: () => void;
} {
  return {
    acquire: (size: number) => globalBufferPool.acquire(size),
    release: (buffer: Float32Array) => globalBufferPool.release(buffer),
    clear: () => globalBufferPool.clear(),
  };
}

// ============================================================================
// Performance Benchmarking Utilities
// ============================================================================

/**
 * Get high-resolution time (cross-platform)
 */
function getTime(): number {
  if (typeof performance !== 'undefined' && performance.now) {
    return performance.now();
  }
  return Date.now();
}

/**
 * Benchmark attention implementation
 */
export function benchmarkAttention(
  attention: MultiHeadAttentionOptimized | FlashAttentionOptimized,
  config: {
    seqLen: number;
    dim: number;
    iterations: number;
    numHeads?: number;
  }
): { avgTimeMs: number; opsPerSecond: number; memoryUsed: number } {
  const { seqLen, dim, iterations, numHeads = 8 } = config;

  // Create test data
  const query = new Float32Array(seqLen * dim);
  const key = new Float32Array(seqLen * dim);
  const value = new Float32Array(seqLen * dim);

  // Initialize with random values
  for (let i = 0; i < query.length; i++) {
    query[i] = Math.random() - 0.5;
    key[i] = Math.random() - 0.5;
    value[i] = Math.random() - 0.5;
  }

  // Warmup
  if (attention instanceof FlashAttentionOptimized) {
    attention.forward(query, key, value, seqLen, dim, numHeads);
  } else if (attention instanceof MultiHeadAttentionOptimized) {
    const q1 = query.slice(0, dim);
    const k1 = key.slice(0, dim);
    const v1 = value.slice(0, dim);
    attention.forward(q1, k1, v1);
  }

  // Benchmark
  const startTime = getTime();

  for (let i = 0; i < iterations; i++) {
    if (attention instanceof FlashAttentionOptimized) {
      const { output } = attention.forward(query, key, value, seqLen, dim, numHeads);
      attention.releaseBuffer(output);
    } else if (attention instanceof MultiHeadAttentionOptimized) {
      const q1 = query.slice(0, dim);
      const k1 = key.slice(0, dim);
      const v1 = value.slice(0, dim);
      const { output } = attention.forward(q1, k1, v1);
      attention.releaseBuffer(output);
    }
  }

  const endTime = getTime();
  const totalTimeMs = endTime - startTime;

  return {
    avgTimeMs: totalTimeMs / iterations,
    opsPerSecond: (iterations * 1000) / totalTimeMs,
    memoryUsed: query.byteLength + key.byteLength + value.byteLength,
  };
}
