/**
 * Browser WASM Attention Wrapper
 *
 * Provides browser-compatible attention mechanisms with:
 * - Lazy WASM loading
 * - Memory management for WASM linear memory
 * - Fallback to JavaScript when WASM unavailable
 * - Loading states and error handling
 *
 * @module browser/AttentionBrowser
 */

export interface AttentionConfig {
  dimension?: number;
  numHeads?: number;
  blockSize?: number;
  curvature?: number;
  useWASM?: boolean;
}

export interface ConsolidationConfig {
  threshold?: number;
  maxClusters?: number;
  minClusterSize?: number;
}

export type LoadingState = 'idle' | 'loading' | 'loaded' | 'error';

/**
 * Browser-compatible attention class with WASM support
 */
export class AttentionBrowser {
  private wasmModule: any = null;
  private loadingState: LoadingState = 'idle';
  private loadError: Error | null = null;
  private config: AttentionConfig;

  constructor(config: AttentionConfig = {}) {
    this.config = {
      dimension: 384,
      numHeads: 4,
      blockSize: 64,
      curvature: -1.0,
      useWASM: true,
      ...config
    };
  }

  /**
   * Get current loading state
   */
  getLoadingState(): LoadingState {
    return this.loadingState;
  }

  /**
   * Get loading error if any
   */
  getError(): Error | null {
    return this.loadError;
  }

  /**
   * Initialize WASM module (lazy loaded)
   */
  async initialize(): Promise<void> {
    if (this.loadingState === 'loaded') return;
    if (this.loadingState === 'loading') {
      // Wait for existing load to complete
      while (this.loadingState === 'loading') {
        await new Promise(resolve => setTimeout(resolve, 50));
      }
      return;
    }

    this.loadingState = 'loading';

    try {
      if (!this.config.useWASM) {
        // Skip WASM loading
        this.loadingState = 'loaded';
        return;
      }

      // Dynamic import of WASM loader
      // @ts-ignore - WASM loader generated during build
      const wasmLoader = await import('../../dist/agentdb.wasm-loader.js');
      this.wasmModule = await wasmLoader.initWASM();
      this.loadingState = 'loaded';
    } catch (error) {
      this.loadError = error instanceof Error ? error : new Error(String(error));
      this.loadingState = 'error';
      console.warn('WASM initialization failed, using fallback:', this.loadError.message);
      // Don't throw - allow fallback to work
    }
  }

  /**
   * Flash Attention - Optimized attention mechanism
   * O(N) memory complexity instead of O(N²)
   *
   * @param query - Query vectors
   * @param keys - Key vectors
   * @param values - Value vectors
   * @returns Attention output
   */
  async flashAttention(
    query: Float32Array,
    keys: Float32Array,
    values: Float32Array
  ): Promise<Float32Array> {
    await this.initialize();

    if (this.wasmModule?.flashAttention) {
      try {
        return this.wasmModule.flashAttention(query, keys, values, this.config);
      } catch (error) {
        console.warn('WASM flash attention failed, using fallback:', error);
      }
    }

    // Fallback to JavaScript implementation
    return this.flashAttentionFallback(query, keys, values);
  }

  /**
   * Hyperbolic Attention - Attention in hyperbolic space
   * Better for hierarchical relationships
   *
   * @param query - Query vector
   * @param keys - Key vectors
   * @returns Similarity scores in hyperbolic space
   */
  async hyperbolicAttention(
    query: Float32Array,
    keys: Float32Array
  ): Promise<Float32Array> {
    await this.initialize();

    if (this.wasmModule?.hyperbolicAttention) {
      try {
        return this.wasmModule.hyperbolicAttention(query, keys, this.config);
      } catch (error) {
        console.warn('WASM hyperbolic attention failed, using fallback:', error);
      }
    }

    // Fallback to JavaScript implementation
    return this.hyperbolicAttentionFallback(query, keys);
  }

  /**
   * Memory Consolidation - Cluster and consolidate similar memories
   *
   * @param memories - Array of memory vectors
   * @param config - Consolidation configuration
   * @returns Consolidated memory clusters
   */
  async consolidateMemories(
    memories: Float32Array[],
    config: ConsolidationConfig = {}
  ): Promise<Array<{
    memory: Float32Array;
    count: number;
    members: Float32Array[];
  }>> {
    await this.initialize();

    const fullConfig = {
      threshold: 0.8,
      maxClusters: 10,
      minClusterSize: 1,
      ...config
    };

    if (this.wasmModule?.memoryConsolidation) {
      try {
        return this.wasmModule.memoryConsolidation(memories, fullConfig);
      } catch (error) {
        console.warn('WASM memory consolidation failed, using fallback:', error);
      }
    }

    // Fallback to JavaScript implementation
    return this.consolidateMemoriesFallback(memories, fullConfig);
  }

  /**
   * Clean up WASM memory
   */
  dispose(): void {
    this.wasmModule = null;
    this.loadingState = 'idle';
    this.loadError = null;
  }

  // ========================================================================
  // Fallback Implementations (Pure JavaScript)
  // ========================================================================

  private flashAttentionFallback(
    query: Float32Array,
    keys: Float32Array,
    values: Float32Array
  ): Float32Array {
    const { dimension = 384 } = this.config;
    const seqLen = keys.length / dimension;
    const output = new Float32Array(query.length);

    for (let i = 0; i < query.length; i += dimension) {
      const q = query.slice(i, i + dimension);
      let sumWeights = 0;
      const weights = new Float32Array(seqLen);

      // Compute attention weights
      for (let j = 0; j < seqLen; j++) {
        const k = keys.slice(j * dimension, (j + 1) * dimension);
        let dot = 0;
        for (let d = 0; d < dimension; d++) {
          dot += q[d] * k[d];
        }
        weights[j] = Math.exp(dot / Math.sqrt(dimension));
        sumWeights += weights[j];
      }

      // Normalize and apply to values
      for (let j = 0; j < seqLen; j++) {
        weights[j] /= (sumWeights || 1);
        const v = values.slice(j * dimension, (j + 1) * dimension);
        for (let d = 0; d < dimension; d++) {
          output[i + d] += weights[j] * v[d];
        }
      }
    }

    return output;
  }

  private hyperbolicAttentionFallback(
    query: Float32Array,
    keys: Float32Array
  ): Float32Array {
    const { curvature = -1.0 } = this.config;
    const k = Math.abs(curvature);
    const similarities = new Float32Array(keys.length / query.length);

    // Hyperbolic distance computation (Poincaré ball model)
    for (let i = 0; i < similarities.length; i++) {
      const offset = i * query.length;
      let dotProduct = 0;
      let normQ = 0;
      let normK = 0;

      for (let j = 0; j < query.length; j++) {
        dotProduct += query[j] * keys[offset + j];
        normQ += query[j] * query[j];
        normK += keys[offset + j] * keys[offset + j];
      }

      // Euclidean distance
      const euclidean = Math.sqrt(normQ + normK - 2 * dotProduct);

      // Poincaré distance
      const poincare = Math.acosh(1 + 2 * k * euclidean * euclidean);

      // Convert to similarity
      similarities[i] = 1 / (1 + poincare);
    }

    return similarities;
  }

  private consolidateMemoriesFallback(
    memories: Float32Array[],
    config: ConsolidationConfig
  ): Array<{
    memory: Float32Array;
    count: number;
    members: Float32Array[];
  }> {
    const { threshold = 0.8, maxClusters = 10, minClusterSize = 1 } = config;
    const consolidated: Array<{
      memory: Float32Array;
      count: number;
      members: Float32Array[];
    }> = [];
    const used = new Set<number>();

    // Simple agglomerative clustering
    for (let i = 0; i < memories.length; i++) {
      if (used.has(i)) continue;

      const cluster: Float32Array[] = [memories[i]];
      used.add(i);

      for (let j = i + 1; j < memories.length; j++) {
        if (used.has(j)) continue;

        // Compute cosine similarity
        const similarity = this.cosineSimilarity(memories[i], memories[j]);

        if (similarity > threshold) {
          cluster.push(memories[j]);
          used.add(j);
        }
      }

      // Only include clusters that meet minimum size
      if (cluster.length >= minClusterSize) {
        // Compute cluster centroid
        const centroid = new Float32Array(memories[i].length);
        for (const mem of cluster) {
          for (let k = 0; k < centroid.length; k++) {
            centroid[k] += mem[k] / cluster.length;
          }
        }

        // Normalize centroid
        let norm = 0;
        for (let k = 0; k < centroid.length; k++) {
          norm += centroid[k] * centroid[k];
        }
        norm = Math.sqrt(norm);
        if (norm > 0) {
          for (let k = 0; k < centroid.length; k++) {
            centroid[k] /= norm;
          }
        }

        consolidated.push({
          memory: centroid,
          count: cluster.length,
          members: cluster
        });
      }

      if (consolidated.length >= maxClusters) break;
    }

    return consolidated;
  }

  private cosineSimilarity(a: Float32Array, b: Float32Array): number {
    let dot = 0;
    let normA = 0;
    let normB = 0;

    for (let i = 0; i < a.length; i++) {
      dot += a[i] * b[i];
      normA += a[i] * a[i];
      normB += b[i] * b[i];
    }

    const denominator = Math.sqrt(normA * normB);
    return denominator > 0 ? dot / denominator : 0;
  }
}

/**
 * Create attention instance with default config
 */
export function createAttention(config?: AttentionConfig): AttentionBrowser {
  return new AttentionBrowser(config);
}

/**
 * Create attention instance optimized for speed
 */
export function createFastAttention(): AttentionBrowser {
  return new AttentionBrowser({
    dimension: 256,
    numHeads: 2,
    blockSize: 32,
    useWASM: true
  });
}

/**
 * Create attention instance optimized for quality
 */
export function createAccurateAttention(): AttentionBrowser {
  return new AttentionBrowser({
    dimension: 768,
    numHeads: 8,
    blockSize: 128,
    useWASM: true
  });
}
