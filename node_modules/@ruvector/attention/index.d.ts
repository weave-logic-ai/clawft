export interface AttentionConfig {
  dim: number;
  numHeads?: number;
  dropout?: number;
}

export function scaledDotProductAttention(query: Float32Array, keys: Float32Array[], values: Float32Array[]): Float32Array;
export function multiHeadAttention(query: Float32Array, keys: Float32Array[], values: Float32Array[], config: AttentionConfig): Float32Array;
export function flashAttention(query: Float32Array, keys: Float32Array[], values: Float32Array[], blockSize?: number): Float32Array;
export function hyperbolicAttention(query: Float32Array, keys: Float32Array[], values: Float32Array[], curvature?: number): Float32Array;
