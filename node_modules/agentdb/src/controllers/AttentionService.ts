/**
 * AttentionService - Advanced Attention Mechanisms for AgentDB
 *
 * Provides state-of-the-art attention mechanisms with runtime detection:
 * - MultiHeadAttention (standard transformer attention)
 * - FlashAttention (memory-efficient attention)
 * - HyperbolicAttention (hyperbolic space attention)
 * - MoEAttention (Mixture-of-Experts attention)
 * - LinearAttention (linear complexity attention)
 *
 * Features:
 * - Automatic runtime detection (Node.js NAPI vs Browser WASM)
 * - Zero-copy Float32Array processing
 * - Graceful fallbacks for unsupported environments
 * - Performance monitoring hooks
 * - Type-safe interfaces
 */

/**
 * Configuration for attention mechanisms
 */
export interface AttentionConfig {
  /** Number of attention heads */
  numHeads: number;
  /** Dimension of each head */
  headDim: number;
  /** Total embedding dimension (usually numHeads * headDim) */
  embedDim: number;
  /** Dropout probability (0-1) */
  dropout?: number;
  /** Whether to use bias in linear projections */
  bias?: boolean;
  /** Use Flash Attention optimization if available */
  useFlash?: boolean;
  /** Use Linear Attention for O(n) complexity */
  useLinear?: boolean;
  /** Use Hyperbolic space for hierarchical data */
  useHyperbolic?: boolean;
  /** Use Mixture-of-Experts routing */
  useMoE?: boolean;
  /** Number of experts for MoE (default: 8) */
  numExperts?: number;
  /** Top-k experts to activate in MoE (default: 2) */
  topK?: number;
}

/**
 * Options for attention operations (alias for AttentionConfig)
 */
export type AttentionOptions = AttentionConfig;

/**
 * Result from attention computation
 */
export interface AttentionResult {
  /** Output embeddings after attention */
  output: Float32Array;
  /** Attention weights (optional, for visualization) */
  weights?: Float32Array;
  /** Execution time in milliseconds */
  executionTimeMs: number;
  /** Which mechanism was used */
  mechanism: 'multi-head' | 'flash' | 'linear' | 'hyperbolic' | 'moe';
  /** Runtime environment */
  runtime: 'napi' | 'wasm' | 'fallback';
}

/**
 * Statistics about attention operations
 */
export interface AttentionStats {
  /** Total attention operations performed */
  totalOps: number;
  /** Average execution time in milliseconds */
  avgExecutionTimeMs: number;
  /** Peak memory usage in bytes */
  peakMemoryBytes: number;
  /** Mechanism usage counts */
  mechanismCounts: Record<string, number>;
  /** Runtime usage counts */
  runtimeCounts: Record<string, number>;
}

/**
 * Performance metrics for attention operations (alias for AttentionStats)
 */
export type AttentionMetrics = AttentionStats;

/**
 * Runtime environment detection
 */
type RuntimeEnvironment = 'nodejs' | 'browser' | 'unknown';

/**
 * Detect the current runtime environment
 */
function detectRuntime(): RuntimeEnvironment {
  // Check for Node.js
  if (typeof process !== 'undefined' && process.versions && process.versions.node) {
    return 'nodejs';
  }

  // Check for browser (with proper type guards)
  if (typeof globalThis !== 'undefined') {
    const global = globalThis as any;
    if (typeof global.window !== 'undefined' && typeof global.document !== 'undefined') {
      return 'browser';
    }
  }

  return 'unknown';
}

/**
 * AttentionService - Main controller for attention mechanisms
 */
export class AttentionService {
  private config: AttentionConfig;
  private runtime: RuntimeEnvironment;
  private napiModule: any = null;
  private wasmModule: any = null;
  private initialized: boolean = false;

  // Performance tracking
  private stats: AttentionStats = {
    totalOps: 0,
    avgExecutionTimeMs: 0,
    peakMemoryBytes: 0,
    mechanismCounts: {},
    runtimeCounts: {}
  };

  constructor(config: AttentionConfig) {
    this.config = {
      dropout: 0.1,
      bias: true,
      useFlash: true,
      useLinear: false,
      useHyperbolic: false,
      useMoE: false,
      numExperts: 8,
      topK: 2,
      ...config
    };
    this.runtime = detectRuntime();
  }

  /**
   * Initialize the attention service
   * Automatically detects and loads the appropriate backend (NAPI or WASM)
   */
  async initialize(): Promise<void> {
    if (this.initialized) {
      return;
    }

    performance.mark('attention-service-init-start');

    try {
      if (this.runtime === 'nodejs') {
        // Try to load NAPI module for Node.js
        await this.loadNAPIModule();
      } else if (this.runtime === 'browser') {
        // Load WASM module for browsers
        await this.loadWASMModule();
      } else {
        console.warn('⚠️  Unknown runtime environment, using fallback implementation');
      }

      this.initialized = true;
      performance.mark('attention-service-init-end');
      performance.measure('attention-service-init', 'attention-service-init-start', 'attention-service-init-end');

      const measure = performance.getEntriesByName('attention-service-init')[0];
      console.log(`✅ AttentionService initialized in ${measure.duration.toFixed(2)}ms (${this.runtime})`);
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      console.error(`❌ AttentionService initialization failed: ${errorMessage}`);
      throw new Error(`Failed to initialize AttentionService: ${errorMessage}`);
    }
  }

  /**
   * Load NAPI module for Node.js runtime
   */
  private async loadNAPIModule(): Promise<void> {
    try {
      // Try to import @ruvector/attention (NAPI bindings)
      // @ts-ignore - Optional dependency
      this.napiModule = await import('@ruvector/attention');
      console.log('✅ Loaded @ruvector/attention NAPI module');
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      console.warn(`⚠️  Failed to load @ruvector/attention: ${errorMessage}`);
      console.warn('   Falling back to JavaScript implementation');
      this.napiModule = null;
    }
  }

  /**
   * Load WASM module for browser runtime
   */
  private async loadWASMModule(): Promise<void> {
    try {
      // Try to import ruvector-attention-wasm
      // @ts-ignore - Optional dependency
      this.wasmModule = await import('ruvector-attention-wasm');
      await this.wasmModule.default(); // Initialize WASM
      console.log('✅ Loaded ruvector-attention-wasm module');
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      console.warn(`⚠️  Failed to load ruvector-attention-wasm: ${errorMessage}`);
      console.warn('   Falling back to JavaScript implementation');
      this.wasmModule = null;
    }
  }

  /**
   * Compute multi-head attention
   *
   * @param query - Query vectors [batchSize * seqLen * embedDim]
   * @param key - Key vectors [batchSize * seqLen * embedDim]
   * @param value - Value vectors [batchSize * seqLen * embedDim]
   * @param mask - Optional attention mask [batchSize * seqLen * seqLen]
   * @returns Attention output and metadata
   */
  async multiHeadAttention(
    query: Float32Array,
    key: Float32Array,
    value: Float32Array,
    mask?: Float32Array
  ): Promise<AttentionResult> {
    if (!this.initialized) {
      await this.initialize();
    }

    performance.mark('mha-start');
    const startTime = Date.now();

    try {
      let output: Float32Array;
      let weights: Float32Array | undefined;
      let runtime: 'napi' | 'wasm' | 'fallback' = 'fallback';

      // Try NAPI first (fastest for Node.js)
      if (this.napiModule && this.napiModule.multiHeadAttention) {
        const result = this.napiModule.multiHeadAttention(
          query,
          key,
          value,
          this.config.numHeads,
          this.config.headDim,
          mask
        );
        output = result.output;
        weights = result.weights;
        runtime = 'napi';
      }
      // Try WASM (for browsers)
      else if (this.wasmModule && this.wasmModule.multiHeadAttention) {
        const result = this.wasmModule.multiHeadAttention(
          query,
          key,
          value,
          this.config.numHeads,
          this.config.headDim,
          mask
        );
        output = result.output;
        weights = result.weights;
        runtime = 'wasm';
      }
      // Fallback to JavaScript implementation
      else {
        const result = this.multiHeadAttentionFallback(query, key, value, mask);
        output = result.output;
        weights = result.weights;
        runtime = 'fallback';
      }

      performance.mark('mha-end');
      performance.measure('mha', 'mha-start', 'mha-end');
      const measure = performance.getEntriesByName('mha')[0];
      const executionTimeMs = measure.duration;

      // Update statistics
      this.updateStats('multi-head', runtime, executionTimeMs, output.length * 4);

      return {
        output,
        weights,
        executionTimeMs,
        mechanism: 'multi-head',
        runtime
      };
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      throw new Error(`Multi-head attention failed: ${errorMessage}`);
    }
  }

  /**
   * Compute Flash Attention (memory-efficient)
   *
   * Flash Attention reduces memory usage from O(n²) to O(n) for sequence length n
   *
   * @param query - Query vectors
   * @param key - Key vectors
   * @param value - Value vectors
   * @param mask - Optional attention mask
   * @returns Attention output and metadata
   */
  async flashAttention(
    query: Float32Array,
    key: Float32Array,
    value: Float32Array,
    mask?: Float32Array
  ): Promise<AttentionResult> {
    if (!this.initialized) {
      await this.initialize();
    }

    performance.mark('flash-start');

    try {
      let output: Float32Array;
      let runtime: 'napi' | 'wasm' | 'fallback' = 'fallback';

      // Try NAPI first
      if (this.napiModule && this.napiModule.flashAttention) {
        output = this.napiModule.flashAttention(
          query,
          key,
          value,
          this.config.numHeads,
          this.config.headDim,
          mask
        );
        runtime = 'napi';
      }
      // Try WASM
      else if (this.wasmModule && this.wasmModule.flashAttention) {
        output = this.wasmModule.flashAttention(
          query,
          key,
          value,
          this.config.numHeads,
          this.config.headDim,
          mask
        );
        runtime = 'wasm';
      }
      // Fallback (same as multi-head for now)
      else {
        const result = this.multiHeadAttentionFallback(query, key, value, mask);
        output = result.output;
        runtime = 'fallback';
      }

      performance.mark('flash-end');
      performance.measure('flash', 'flash-start', 'flash-end');
      const measure = performance.getEntriesByName('flash')[0];
      const executionTimeMs = measure.duration;

      // Update statistics
      this.updateStats('flash', runtime, executionTimeMs, output.length * 4);

      return {
        output,
        executionTimeMs,
        mechanism: 'flash',
        runtime
      };
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      throw new Error(`Flash attention failed: ${errorMessage}`);
    }
  }

  /**
   * Compute Linear Attention (O(n) complexity)
   *
   * Linear attention approximates standard attention with linear complexity
   *
   * @param query - Query vectors
   * @param key - Key vectors
   * @param value - Value vectors
   * @returns Attention output and metadata
   */
  async linearAttention(
    query: Float32Array,
    key: Float32Array,
    value: Float32Array
  ): Promise<AttentionResult> {
    if (!this.initialized) {
      await this.initialize();
    }

    performance.mark('linear-start');

    try {
      let output: Float32Array;
      let runtime: 'napi' | 'wasm' | 'fallback' = 'fallback';

      // Try NAPI first
      if (this.napiModule && this.napiModule.linearAttention) {
        output = this.napiModule.linearAttention(
          query,
          key,
          value,
          this.config.numHeads,
          this.config.headDim
        );
        runtime = 'napi';
      }
      // Try WASM
      else if (this.wasmModule && this.wasmModule.linearAttention) {
        output = this.wasmModule.linearAttention(
          query,
          key,
          value,
          this.config.numHeads,
          this.config.headDim
        );
        runtime = 'wasm';
      }
      // Fallback
      else {
        output = this.linearAttentionFallback(query, key, value);
        runtime = 'fallback';
      }

      performance.mark('linear-end');
      performance.measure('linear', 'linear-start', 'linear-end');
      const measure = performance.getEntriesByName('linear')[0];
      const executionTimeMs = measure.duration;

      // Update statistics
      this.updateStats('linear', runtime, executionTimeMs, output.length * 4);

      return {
        output,
        executionTimeMs,
        mechanism: 'linear',
        runtime
      };
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      throw new Error(`Linear attention failed: ${errorMessage}`);
    }
  }

  /**
   * Compute Hyperbolic Attention (for hierarchical data)
   *
   * Hyperbolic attention operates in hyperbolic space, suitable for tree-like structures
   *
   * @param query - Query vectors
   * @param key - Key vectors
   * @param value - Value vectors
   * @param curvature - Hyperbolic space curvature (default: -1.0)
   * @returns Attention output and metadata
   */
  async hyperbolicAttention(
    query: Float32Array,
    key: Float32Array,
    value: Float32Array,
    curvature: number = -1.0
  ): Promise<AttentionResult> {
    if (!this.initialized) {
      await this.initialize();
    }

    performance.mark('hyperbolic-start');

    try {
      let output: Float32Array;
      let runtime: 'napi' | 'wasm' | 'fallback' = 'fallback';

      // Try NAPI first
      if (this.napiModule && this.napiModule.hyperbolicAttention) {
        output = this.napiModule.hyperbolicAttention(
          query,
          key,
          value,
          this.config.numHeads,
          this.config.headDim,
          curvature
        );
        runtime = 'napi';
      }
      // Try WASM
      else if (this.wasmModule && this.wasmModule.hyperbolicAttention) {
        output = this.wasmModule.hyperbolicAttention(
          query,
          key,
          value,
          this.config.numHeads,
          this.config.headDim,
          curvature
        );
        runtime = 'wasm';
      }
      // Fallback (use standard attention)
      else {
        const result = this.multiHeadAttentionFallback(query, key, value);
        output = result.output;
        runtime = 'fallback';
      }

      performance.mark('hyperbolic-end');
      performance.measure('hyperbolic', 'hyperbolic-start', 'hyperbolic-end');
      const measure = performance.getEntriesByName('hyperbolic')[0];
      const executionTimeMs = measure.duration;

      // Update statistics
      this.updateStats('hyperbolic', runtime, executionTimeMs, output.length * 4);

      return {
        output,
        executionTimeMs,
        mechanism: 'hyperbolic',
        runtime
      };
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      throw new Error(`Hyperbolic attention failed: ${errorMessage}`);
    }
  }

  /**
   * Compute Mixture-of-Experts (MoE) Attention
   *
   * MoE routes inputs to different expert attention mechanisms
   *
   * @param query - Query vectors
   * @param key - Key vectors
   * @param value - Value vectors
   * @param mask - Optional attention mask
   * @returns Attention output and metadata
   */
  async moeAttention(
    query: Float32Array,
    key: Float32Array,
    value: Float32Array,
    mask?: Float32Array
  ): Promise<AttentionResult> {
    if (!this.initialized) {
      await this.initialize();
    }

    performance.mark('moe-start');

    try {
      let output: Float32Array;
      let runtime: 'napi' | 'wasm' | 'fallback' = 'fallback';

      const numExperts = this.config.numExperts || 8;
      const topK = this.config.topK || 2;

      // Try NAPI first
      if (this.napiModule && this.napiModule.moeAttention) {
        output = this.napiModule.moeAttention(
          query,
          key,
          value,
          this.config.numHeads,
          this.config.headDim,
          numExperts,
          topK,
          mask
        );
        runtime = 'napi';
      }
      // Try WASM
      else if (this.wasmModule && this.wasmModule.moeAttention) {
        output = this.wasmModule.moeAttention(
          query,
          key,
          value,
          this.config.numHeads,
          this.config.headDim,
          numExperts,
          topK,
          mask
        );
        runtime = 'wasm';
      }
      // Fallback (use standard attention)
      else {
        const result = this.multiHeadAttentionFallback(query, key, value, mask);
        output = result.output;
        runtime = 'fallback';
      }

      performance.mark('moe-end');
      performance.measure('moe', 'moe-start', 'moe-end');
      const measure = performance.getEntriesByName('moe')[0];
      const executionTimeMs = measure.duration;

      // Update statistics
      this.updateStats('moe', runtime, executionTimeMs, output.length * 4);

      return {
        output,
        executionTimeMs,
        mechanism: 'moe',
        runtime
      };
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      throw new Error(`MoE attention failed: ${errorMessage}`);
    }
  }

  /**
   * Fallback JavaScript implementation of multi-head attention
   * Used when native modules are not available
   */
  private multiHeadAttentionFallback(
    query: Float32Array,
    key: Float32Array,
    value: Float32Array,
    mask?: Float32Array
  ): { output: Float32Array; weights?: Float32Array } {
    const { numHeads, headDim, embedDim } = this.config;
    const seqLen = Math.floor(query.length / embedDim);
    const batchSize = 1; // Simplified for fallback

    // Simple scaled dot-product attention
    const scale = 1.0 / Math.sqrt(headDim);
    const output = new Float32Array(query.length);

    for (let i = 0; i < seqLen; i++) {
      for (let d = 0; d < embedDim; d++) {
        let sum = 0;
        let weightSum = 0;

        for (let j = 0; j < seqLen; j++) {
          // Compute attention score
          let score = 0;
          for (let k = 0; k < headDim; k++) {
            const qIdx = i * embedDim + k;
            const kIdx = j * embedDim + k;
            score += query[qIdx] * key[kIdx];
          }
          score *= scale;

          // Apply mask if provided
          if (mask && mask[i * seqLen + j] === 0) {
            score = -Infinity;
          }

          // Softmax (simplified)
          const weight = Math.exp(score);
          const vIdx = j * embedDim + d;
          sum += weight * value[vIdx];
          weightSum += weight;
        }

        output[i * embedDim + d] = weightSum > 0 ? sum / weightSum : 0;
      }
    }

    return { output };
  }

  /**
   * Fallback JavaScript implementation of linear attention
   */
  private linearAttentionFallback(
    query: Float32Array,
    key: Float32Array,
    value: Float32Array
  ): Float32Array {
    // Simplified linear attention using feature maps
    const { embedDim } = this.config;
    const seqLen = Math.floor(query.length / embedDim);
    const output = new Float32Array(query.length);

    // Apply feature map (elu + 1)
    const featureMap = (x: number) => x > 0 ? x + 1 : Math.exp(x);

    for (let i = 0; i < seqLen; i++) {
      for (let d = 0; d < embedDim; d++) {
        let numerator = 0;
        let denominator = 0;

        for (let j = 0; j < seqLen; j++) {
          const qVal = featureMap(query[i * embedDim + d]);
          const kVal = featureMap(key[j * embedDim + d]);
          const vVal = value[j * embedDim + d];

          numerator += qVal * kVal * vVal;
          denominator += qVal * kVal;
        }

        output[i * embedDim + d] = denominator > 0 ? numerator / denominator : 0;
      }
    }

    return output;
  }

  /**
   * Update performance statistics
   */
  private updateStats(
    mechanism: string,
    runtime: string,
    executionTimeMs: number,
    memoryBytes: number
  ): void {
    this.stats.totalOps++;

    // Update average execution time
    const prevTotal = this.stats.avgExecutionTimeMs * (this.stats.totalOps - 1);
    this.stats.avgExecutionTimeMs = (prevTotal + executionTimeMs) / this.stats.totalOps;

    // Update peak memory
    if (memoryBytes > this.stats.peakMemoryBytes) {
      this.stats.peakMemoryBytes = memoryBytes;
    }

    // Update mechanism counts
    this.stats.mechanismCounts[mechanism] = (this.stats.mechanismCounts[mechanism] || 0) + 1;

    // Update runtime counts
    this.stats.runtimeCounts[runtime] = (this.stats.runtimeCounts[runtime] || 0) + 1;
  }

  /**
   * Get performance statistics
   */
  getStats(): AttentionStats {
    return { ...this.stats };
  }

  /**
   * Reset performance statistics
   */
  resetStats(): void {
    this.stats = {
      totalOps: 0,
      avgExecutionTimeMs: 0,
      peakMemoryBytes: 0,
      mechanismCounts: {},
      runtimeCounts: {}
    };
  }

  /**
   * Get service information
   */
  getInfo(): {
    initialized: boolean;
    runtime: RuntimeEnvironment;
    hasNAPI: boolean;
    hasWASM: boolean;
    config: AttentionConfig;
  } {
    return {
      initialized: this.initialized,
      runtime: this.runtime,
      hasNAPI: this.napiModule !== null,
      hasWASM: this.wasmModule !== null,
      config: { ...this.config }
    };
  }
}
