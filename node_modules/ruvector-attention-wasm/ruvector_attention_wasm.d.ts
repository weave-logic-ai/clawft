/* tslint:disable */
/* eslint-disable */

export class WasmAdam {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new Adam optimizer
   *
   * # Arguments
   * * `param_count` - Number of parameters
   * * `learning_rate` - Learning rate
   */
  constructor(param_count: number, learning_rate: number);
  /**
   * Perform optimization step
   *
   * # Arguments
   * * `params` - Current parameter values (will be updated in-place)
   * * `gradients` - Gradient values
   */
  step(params: Float32Array, gradients: Float32Array): void;
  /**
   * Reset optimizer state
   */
  reset(): void;
  /**
   * Get current learning rate
   */
  learning_rate: number;
}

export class WasmAdamW {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new AdamW optimizer
   *
   * # Arguments
   * * `param_count` - Number of parameters
   * * `learning_rate` - Learning rate
   * * `weight_decay` - Weight decay coefficient
   */
  constructor(param_count: number, learning_rate: number, weight_decay: number);
  /**
   * Perform optimization step with weight decay
   */
  step(params: Float32Array, gradients: Float32Array): void;
  /**
   * Reset optimizer state
   */
  reset(): void;
  /**
   * Get weight decay
   */
  readonly weight_decay: number;
  /**
   * Get current learning rate
   */
  learning_rate: number;
}

export class WasmFlashAttention {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new flash attention instance
   *
   * # Arguments
   * * `dim` - Embedding dimension
   * * `block_size` - Block size for tiling
   */
  constructor(dim: number, block_size: number);
  /**
   * Compute flash attention
   */
  compute(query: Float32Array, keys: any, values: any): Float32Array;
}

export class WasmHyperbolicAttention {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new hyperbolic attention instance
   *
   * # Arguments
   * * `dim` - Embedding dimension
   * * `curvature` - Hyperbolic curvature parameter
   */
  constructor(dim: number, curvature: number);
  /**
   * Compute hyperbolic attention
   */
  compute(query: Float32Array, keys: any, values: any): Float32Array;
  /**
   * Get the curvature
   */
  readonly curvature: number;
}

export class WasmInfoNCELoss {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new InfoNCE loss instance
   *
   * # Arguments
   * * `temperature` - Temperature parameter for softmax
   */
  constructor(temperature: number);
  /**
   * Compute InfoNCE loss
   *
   * # Arguments
   * * `anchor` - Anchor embedding
   * * `positive` - Positive example embedding
   * * `negatives` - Array of negative example embeddings
   */
  compute(anchor: Float32Array, positive: Float32Array, negatives: any): number;
}

export class WasmLRScheduler {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new learning rate scheduler with warmup and cosine decay
   *
   * # Arguments
   * * `initial_lr` - Initial learning rate
   * * `warmup_steps` - Number of warmup steps
   * * `total_steps` - Total training steps
   */
  constructor(initial_lr: number, warmup_steps: number, total_steps: number);
  /**
   * Advance to next step
   */
  step(): void;
  /**
   * Reset scheduler
   */
  reset(): void;
  /**
   * Get learning rate for current step
   */
  get_lr(): number;
}

export class WasmLinearAttention {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new linear attention instance
   *
   * # Arguments
   * * `dim` - Embedding dimension
   * * `num_features` - Number of random features
   */
  constructor(dim: number, num_features: number);
  /**
   * Compute linear attention
   */
  compute(query: Float32Array, keys: any, values: any): Float32Array;
}

export class WasmLocalGlobalAttention {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new local-global attention instance
   *
   * # Arguments
   * * `dim` - Embedding dimension
   * * `local_window` - Size of local attention window
   * * `global_tokens` - Number of global attention tokens
   */
  constructor(dim: number, local_window: number, global_tokens: number);
  /**
   * Compute local-global attention
   */
  compute(query: Float32Array, keys: any, values: any): Float32Array;
}

export class WasmMoEAttention {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new MoE attention instance
   *
   * # Arguments
   * * `dim` - Embedding dimension
   * * `num_experts` - Number of expert attention mechanisms
   * * `top_k` - Number of experts to use per query
   */
  constructor(dim: number, num_experts: number, top_k: number);
  /**
   * Compute MoE attention
   */
  compute(query: Float32Array, keys: any, values: any): Float32Array;
}

export class WasmMultiHeadAttention {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new multi-head attention instance
   *
   * # Arguments
   * * `dim` - Embedding dimension
   * * `num_heads` - Number of attention heads
   */
  constructor(dim: number, num_heads: number);
  /**
   * Compute multi-head attention
   */
  compute(query: Float32Array, keys: any, values: any): Float32Array;
  /**
   * Get the dimension
   */
  readonly dim: number;
  /**
   * Get the number of heads
   */
  readonly num_heads: number;
}

export class WasmSGD {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new SGD optimizer
   *
   * # Arguments
   * * `param_count` - Number of parameters
   * * `learning_rate` - Learning rate
   * * `momentum` - Momentum coefficient (default: 0)
   */
  constructor(param_count: number, learning_rate: number, momentum?: number | null);
  /**
   * Perform optimization step
   */
  step(params: Float32Array, gradients: Float32Array): void;
  /**
   * Reset optimizer state
   */
  reset(): void;
  /**
   * Get current learning rate
   */
  learning_rate: number;
}

/**
 * Compute attention weights from scores
 */
export function attention_weights(scores: Float32Array, temperature?: number | null): void;

/**
 * Get information about available attention mechanisms
 */
export function available_mechanisms(): any;

/**
 * Batch normalize vectors
 */
export function batch_normalize(vectors: any, epsilon?: number | null): Float32Array;

/**
 * Compute cosine similarity between two vectors
 */
export function cosine_similarity(a: Float32Array, b: Float32Array): number;

/**
 * Initialize the WASM module with panic hook
 */
export function init(): void;

/**
 * Compute L2 norm of a vector
 */
export function l2_norm(vec: Float32Array): number;

/**
 * Log a message to the browser console
 */
export function log(message: string): void;

/**
 * Log an error to the browser console
 */
export function log_error(message: string): void;

/**
 * Normalize a vector to unit length
 */
export function normalize(vec: Float32Array): void;

/**
 * Compute pairwise distances between vectors
 */
export function pairwise_distances(vectors: any): Float32Array;

/**
 * Generate random orthogonal matrix (for initialization)
 */
export function random_orthogonal_matrix(dim: number): Float32Array;

/**
 * Compute scaled dot-product attention
 *
 * # Arguments
 * * `query` - Query vector as Float32Array
 * * `keys` - Array of key vectors
 * * `values` - Array of value vectors
 * * `scale` - Optional scaling factor (defaults to 1/sqrt(dim))
 */
export function scaled_dot_attention(query: Float32Array, keys: any, values: any, scale?: number | null): Float32Array;

/**
 * Compute softmax of a vector
 */
export function softmax(vec: Float32Array): void;

/**
 * Get the version of the ruvector-attention-wasm crate
 */
export function version(): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_wasmadam_free: (a: number, b: number) => void;
  readonly __wbg_wasmadamw_free: (a: number, b: number) => void;
  readonly __wbg_wasmflashattention_free: (a: number, b: number) => void;
  readonly __wbg_wasmhyperbolicattention_free: (a: number, b: number) => void;
  readonly __wbg_wasminfonceloss_free: (a: number, b: number) => void;
  readonly __wbg_wasmlinearattention_free: (a: number, b: number) => void;
  readonly __wbg_wasmmoeattention_free: (a: number, b: number) => void;
  readonly __wbg_wasmmultiheadattention_free: (a: number, b: number) => void;
  readonly __wbg_wasmsgd_free: (a: number, b: number) => void;
  readonly attention_weights: (a: number, b: number, c: number, d: number) => void;
  readonly available_mechanisms: () => number;
  readonly batch_normalize: (a: number, b: number, c: number) => void;
  readonly cosine_similarity: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly l2_norm: (a: number, b: number) => number;
  readonly log: (a: number, b: number) => void;
  readonly log_error: (a: number, b: number) => void;
  readonly normalize: (a: number, b: number, c: number, d: number) => void;
  readonly pairwise_distances: (a: number, b: number) => void;
  readonly random_orthogonal_matrix: (a: number, b: number) => void;
  readonly scaled_dot_attention: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly softmax: (a: number, b: number, c: number) => void;
  readonly version: (a: number) => void;
  readonly wasmadam_learning_rate: (a: number) => number;
  readonly wasmadam_new: (a: number, b: number) => number;
  readonly wasmadam_reset: (a: number) => void;
  readonly wasmadam_set_learning_rate: (a: number, b: number) => void;
  readonly wasmadam_step: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly wasmadamw_new: (a: number, b: number, c: number) => number;
  readonly wasmadamw_reset: (a: number) => void;
  readonly wasmadamw_step: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly wasmadamw_weight_decay: (a: number) => number;
  readonly wasmflashattention_compute: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly wasmflashattention_new: (a: number, b: number) => number;
  readonly wasmhyperbolicattention_compute: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly wasmhyperbolicattention_curvature: (a: number) => number;
  readonly wasmhyperbolicattention_new: (a: number, b: number) => number;
  readonly wasminfonceloss_compute: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
  readonly wasminfonceloss_new: (a: number) => number;
  readonly wasmlinearattention_compute: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly wasmlinearattention_new: (a: number, b: number) => number;
  readonly wasmlocalglobalattention_compute: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly wasmlocalglobalattention_new: (a: number, b: number, c: number) => number;
  readonly wasmlrscheduler_get_lr: (a: number) => number;
  readonly wasmlrscheduler_new: (a: number, b: number, c: number) => number;
  readonly wasmlrscheduler_reset: (a: number) => void;
  readonly wasmlrscheduler_step: (a: number) => void;
  readonly wasmmoeattention_compute: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly wasmmoeattention_new: (a: number, b: number, c: number) => number;
  readonly wasmmultiheadattention_compute: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly wasmmultiheadattention_dim: (a: number) => number;
  readonly wasmmultiheadattention_new: (a: number, b: number, c: number) => void;
  readonly wasmmultiheadattention_num_heads: (a: number) => number;
  readonly wasmsgd_learning_rate: (a: number) => number;
  readonly wasmsgd_new: (a: number, b: number, c: number) => number;
  readonly wasmsgd_reset: (a: number) => void;
  readonly wasmsgd_set_learning_rate: (a: number, b: number) => void;
  readonly wasmsgd_step: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly init: () => void;
  readonly wasmadamw_set_learning_rate: (a: number, b: number) => void;
  readonly wasmadamw_learning_rate: (a: number) => number;
  readonly __wbg_wasmlocalglobalattention_free: (a: number, b: number) => void;
  readonly __wbg_wasmlrscheduler_free: (a: number, b: number) => void;
  readonly __wbindgen_export: (a: number, b: number) => number;
  readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_export3: (a: number) => void;
  readonly __wbindgen_export4: (a: number, b: number, c: number) => void;
  readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
