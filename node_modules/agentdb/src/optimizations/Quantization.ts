/**
 * Vector Quantization Module for AgentDB
 *
 * Provides memory-efficient vector storage through quantization techniques:
 * - Scalar Quantization (4-bit and 8-bit)
 * - Product Quantization with trainable codebooks
 * - Quantized Vector Store with asymmetric distance computation
 *
 * @module optimizations/Quantization
 */

// ============================================================================
// Type Definitions
// ============================================================================

/**
 * Supported quantization types
 */
export type QuantizationType = 'scalar-4bit' | 'scalar-8bit' | 'product';

/**
 * Configuration for quantization operations
 */
export interface QuantizationConfig {
  /** Type of quantization to apply */
  type: QuantizationType;
  /** Number of subvectors for product quantization (default: 8) */
  numSubvectors?: number;
  /** Number of centroids per subvector for product quantization (default: 256) */
  numCentroids?: number;
  /** Number of training iterations for product quantization (default: 25) */
  trainingIterations?: number;
}

/**
 * Container for quantized vector data
 */
export interface QuantizedVectors {
  /** The quantization type used */
  type: QuantizationType;
  /** Original vector dimension */
  dimension: number;
  /** Number of vectors stored */
  count: number;
  /** Quantized data buffer */
  data: Uint8Array;
  /** Minimum values per dimension (for scalar quantization) */
  mins?: Float32Array;
  /** Maximum values per dimension (for scalar quantization) */
  maxs?: Float32Array;
  /** Codebooks for product quantization */
  codebooks?: Float32Array[];
  /** Number of subvectors (for product quantization) */
  numSubvectors?: number;
  /** Subvector dimension (for product quantization) */
  subvectorDim?: number;
}

/**
 * Search result from quantized vector store
 */
export interface SearchResult {
  /** Unique identifier of the vector */
  id: string;
  /** Distance/similarity score */
  distance: number;
  /** Original index in the store */
  index: number;
}

// ============================================================================
// Scalar Quantization
// ============================================================================

/**
 * Scalar quantization reduces memory by mapping floating-point values
 * to lower-bit integer representations.
 *
 * @example
 * ```typescript
 * const sq = new ScalarQuantization();
 * const vectors = [new Float32Array([0.1, 0.5, 0.9])];
 * const quantized = sq.quantize(vectors, 8);
 * const restored = sq.dequantize(quantized);
 * ```
 */
export class ScalarQuantization {
  /**
   * Quantize vectors to reduced bit representation
   *
   * @param vectors - Array of vectors to quantize
   * @param bits - Target bit depth (4 or 8)
   * @returns Quantized vector container
   */
  quantize(vectors: Float32Array[], bits: 4 | 8): QuantizedVectors {
    if (vectors.length === 0) {
      throw new Error('Cannot quantize empty vector array');
    }

    const dimension = vectors[0].length;
    const maxVal = bits === 8 ? 255 : 15;

    // Compute min/max per dimension for optimal range utilization
    const mins = new Float32Array(dimension).fill(Infinity);
    const maxs = new Float32Array(dimension).fill(-Infinity);

    for (let i = 0; i < vectors.length; i++) {
      const vec = vectors[i];
      for (let d = 0; d < dimension; d++) {
        if (vec[d] < mins[d]) mins[d] = vec[d];
        if (vec[d] > maxs[d]) maxs[d] = vec[d];
      }
    }

    // Compute scales per dimension
    const scales = new Float32Array(dimension);
    for (let d = 0; d < dimension; d++) {
      const range = maxs[d] - mins[d];
      scales[d] = range > 0 ? maxVal / range : 0;
    }

    // Allocate output buffer
    const bytesPerVector = bits === 8 ? dimension : Math.ceil(dimension / 2);
    const data = new Uint8Array(vectors.length * bytesPerVector);

    // Quantize each vector
    for (let i = 0; i < vectors.length; i++) {
      const vec = vectors[i];
      const offset = i * bytesPerVector;

      if (bits === 8) {
        // 8-bit: one byte per value
        for (let d = 0; d < dimension; d++) {
          const normalized = (vec[d] - mins[d]) * scales[d];
          data[offset + d] = Math.min(maxVal, Math.max(0, Math.round(normalized)));
        }
      } else {
        // 4-bit: pack two values per byte
        for (let d = 0; d < dimension; d += 2) {
          const val1 = Math.min(maxVal, Math.max(0, Math.round((vec[d] - mins[d]) * scales[d])));
          const val2 =
            d + 1 < dimension
              ? Math.min(maxVal, Math.max(0, Math.round((vec[d + 1] - mins[d + 1]) * scales[d + 1])))
              : 0;
          data[offset + Math.floor(d / 2)] = (val1 << 4) | val2;
        }
      }
    }

    return {
      type: bits === 8 ? 'scalar-8bit' : 'scalar-4bit',
      dimension,
      count: vectors.length,
      data,
      mins,
      maxs,
    };
  }

  /**
   * Dequantize vectors back to floating-point representation
   *
   * @param quantized - Quantized vector container
   * @returns Array of reconstructed vectors
   */
  dequantize(quantized: QuantizedVectors): Float32Array[] {
    const { type, dimension, count, data, mins, maxs } = quantized;

    if (!mins || !maxs) {
      throw new Error('Scalar quantized vectors require mins and maxs');
    }

    const bits = type === 'scalar-8bit' ? 8 : 4;
    const maxVal = bits === 8 ? 255 : 15;
    const bytesPerVector = bits === 8 ? dimension : Math.ceil(dimension / 2);

    // Compute inverse scales
    const invScales = new Float32Array(dimension);
    for (let d = 0; d < dimension; d++) {
      const range = maxs[d] - mins[d];
      invScales[d] = range / maxVal;
    }

    const vectors: Float32Array[] = new Array(count);

    for (let i = 0; i < count; i++) {
      const vec = new Float32Array(dimension);
      const offset = i * bytesPerVector;

      if (bits === 8) {
        // 8-bit: one byte per value
        for (let d = 0; d < dimension; d++) {
          vec[d] = data[offset + d] * invScales[d] + mins[d];
        }
      } else {
        // 4-bit: unpack two values per byte
        for (let d = 0; d < dimension; d += 2) {
          const packed = data[offset + Math.floor(d / 2)];
          vec[d] = ((packed >> 4) & 0x0f) * invScales[d] + mins[d];
          if (d + 1 < dimension) {
            vec[d + 1] = (packed & 0x0f) * invScales[d + 1] + mins[d + 1];
          }
        }
      }

      vectors[i] = vec;
    }

    return vectors;
  }

  /**
   * Compute approximate squared Euclidean distance using quantized representation
   *
   * @param quantized - Quantized vectors
   * @param index - Index of vector in quantized store
   * @param query - Full-precision query vector
   * @returns Approximate squared distance
   */
  computeDistance(quantized: QuantizedVectors, index: number, query: Float32Array): number {
    const { type, dimension, data, mins, maxs } = quantized;

    if (!mins || !maxs) {
      throw new Error('Scalar quantized vectors require mins and maxs');
    }

    const bits = type === 'scalar-8bit' ? 8 : 4;
    const maxVal = bits === 8 ? 255 : 15;
    const bytesPerVector = bits === 8 ? dimension : Math.ceil(dimension / 2);
    const offset = index * bytesPerVector;

    // Compute inverse scales
    let distanceSq = 0;

    if (bits === 8) {
      for (let d = 0; d < dimension; d++) {
        const range = maxs[d] - mins[d];
        const invScale = range / maxVal;
        const reconstructed = data[offset + d] * invScale + mins[d];
        const diff = reconstructed - query[d];
        distanceSq += diff * diff;
      }
    } else {
      for (let d = 0; d < dimension; d += 2) {
        const packed = data[offset + Math.floor(d / 2)];

        const range1 = maxs[d] - mins[d];
        const invScale1 = range1 / maxVal;
        const reconstructed1 = ((packed >> 4) & 0x0f) * invScale1 + mins[d];
        const diff1 = reconstructed1 - query[d];
        distanceSq += diff1 * diff1;

        if (d + 1 < dimension) {
          const range2 = maxs[d + 1] - mins[d + 1];
          const invScale2 = range2 / maxVal;
          const reconstructed2 = (packed & 0x0f) * invScale2 + mins[d + 1];
          const diff2 = reconstructed2 - query[d + 1];
          distanceSq += diff2 * diff2;
        }
      }
    }

    return distanceSq;
  }
}

// ============================================================================
// Product Quantization
// ============================================================================

/**
 * Product quantization divides vectors into subvectors and learns
 * codebooks for each subspace, enabling high compression ratios.
 *
 * @example
 * ```typescript
 * const pq = new ProductQuantization(8, 256);
 * pq.train(trainingVectors);
 * const encoded = pq.encode(vectors);
 * const decoded = pq.decode(encoded);
 * ```
 */
export class ProductQuantization {
  /** Number of subvectors to divide each vector into */
  private readonly numSubvectors: number;
  /** Number of centroids per subvector codebook */
  private readonly numCentroids: number;
  /** Maximum training iterations */
  private readonly maxIterations: number;
  /** Codebooks: one per subvector, each containing numCentroids centroids */
  private codebooks: Float32Array[] = [];
  /** Dimension of each subvector */
  private subvectorDim = 0;
  /** Total vector dimension */
  private dimension = 0;
  /** Whether the model has been trained */
  private trained = false;

  /**
   * Create a product quantization instance
   *
   * @param numSubvectors - Number of subvectors (default: 8)
   * @param numCentroids - Centroids per subvector (default: 256, max 256 for byte encoding)
   * @param maxIterations - Maximum k-means iterations (default: 25)
   */
  constructor(numSubvectors = 8, numCentroids = 256, maxIterations = 25) {
    if (numCentroids > 256) {
      throw new Error('Number of centroids cannot exceed 256 for byte encoding');
    }
    this.numSubvectors = numSubvectors;
    this.numCentroids = numCentroids;
    this.maxIterations = maxIterations;
  }

  /**
   * Train codebooks from sample vectors using k-means clustering
   *
   * @param vectors - Training vectors
   */
  train(vectors: Float32Array[]): void {
    if (vectors.length === 0) {
      throw new Error('Cannot train on empty vector set');
    }

    this.dimension = vectors[0].length;
    if (this.dimension % this.numSubvectors !== 0) {
      throw new Error(
        `Vector dimension ${this.dimension} must be divisible by numSubvectors ${this.numSubvectors}`
      );
    }

    this.subvectorDim = this.dimension / this.numSubvectors;
    this.codebooks = new Array(this.numSubvectors);

    // Train each subvector codebook independently
    for (let m = 0; m < this.numSubvectors; m++) {
      // Extract subvectors for this segment
      const subvectors: Float32Array[] = vectors.map((vec) => {
        const start = m * this.subvectorDim;
        return vec.slice(start, start + this.subvectorDim) as Float32Array;
      });

      // Train codebook using k-means
      this.codebooks[m] = this.trainCodebook(subvectors);
    }

    this.trained = true;
  }

  /**
   * Train a single codebook using k-means clustering
   *
   * @param subvectors - Subvectors for this segment
   * @returns Codebook as flattened Float32Array
   */
  private trainCodebook(subvectors: Float32Array[]): Float32Array {
    const k = Math.min(this.numCentroids, subvectors.length);
    const dim = this.subvectorDim;

    // Initialize centroids using k-means++ strategy
    const centroids = new Float32Array(k * dim);
    const usedIndices = new Set<number>();

    // First centroid: random
    let idx = Math.floor(Math.random() * subvectors.length);
    usedIndices.add(idx);
    centroids.set(subvectors[idx], 0);

    // Remaining centroids: probability proportional to squared distance
    for (let c = 1; c < k; c++) {
      const distances = new Float32Array(subvectors.length);
      let totalDist = 0;

      for (let i = 0; i < subvectors.length; i++) {
        if (usedIndices.has(i)) {
          distances[i] = 0;
          continue;
        }

        // Find minimum distance to existing centroids
        let minDist = Infinity;
        for (let j = 0; j < c; j++) {
          const dist = this.squaredDistance(
            subvectors[i],
            centroids.subarray(j * dim, (j + 1) * dim) as Float32Array
          );
          if (dist < minDist) minDist = dist;
        }
        distances[i] = minDist;
        totalDist += minDist;
      }

      // Select next centroid proportionally
      let threshold = Math.random() * totalDist;
      for (let i = 0; i < subvectors.length; i++) {
        threshold -= distances[i];
        if (threshold <= 0) {
          usedIndices.add(i);
          centroids.set(subvectors[i], c * dim);
          break;
        }
      }
    }

    // K-means iterations
    const assignments = new Uint16Array(subvectors.length);
    const counts = new Uint32Array(k);
    const sums = new Float32Array(k * dim);

    for (let iter = 0; iter < this.maxIterations; iter++) {
      // Assignment step
      let changed = false;
      for (let i = 0; i < subvectors.length; i++) {
        let bestCentroid = 0;
        let bestDist = Infinity;

        for (let c = 0; c < k; c++) {
          const dist = this.squaredDistance(
            subvectors[i],
            centroids.subarray(c * dim, (c + 1) * dim) as Float32Array
          );
          if (dist < bestDist) {
            bestDist = dist;
            bestCentroid = c;
          }
        }

        if (assignments[i] !== bestCentroid) {
          assignments[i] = bestCentroid;
          changed = true;
        }
      }

      if (!changed) break;

      // Update step
      counts.fill(0);
      sums.fill(0);

      for (let i = 0; i < subvectors.length; i++) {
        const c = assignments[i];
        counts[c]++;
        for (let d = 0; d < dim; d++) {
          sums[c * dim + d] += subvectors[i][d];
        }
      }

      for (let c = 0; c < k; c++) {
        if (counts[c] > 0) {
          for (let d = 0; d < dim; d++) {
            centroids[c * dim + d] = sums[c * dim + d] / counts[c];
          }
        }
      }
    }

    return centroids;
  }

  /**
   * Encode vectors using trained codebooks
   *
   * @param vectors - Vectors to encode
   * @returns Quantized vector container
   */
  encode(vectors: Float32Array[]): QuantizedVectors {
    if (!this.trained) {
      throw new Error('Product quantization model must be trained before encoding');
    }

    const data = new Uint8Array(vectors.length * this.numSubvectors);

    for (let i = 0; i < vectors.length; i++) {
      const vec = vectors[i];

      for (let m = 0; m < this.numSubvectors; m++) {
        const start = m * this.subvectorDim;
        const subvector = vec.slice(start, start + this.subvectorDim);
        const codebook = this.codebooks[m];

        // Find nearest centroid
        let bestIdx = 0;
        let bestDist = Infinity;

        for (let c = 0; c < this.numCentroids; c++) {
          const centroid = codebook.subarray(
            c * this.subvectorDim,
            (c + 1) * this.subvectorDim
          ) as Float32Array;
          const dist = this.squaredDistance(subvector as Float32Array, centroid);

          if (dist < bestDist) {
            bestDist = dist;
            bestIdx = c;
          }
        }

        data[i * this.numSubvectors + m] = bestIdx;
      }
    }

    return {
      type: 'product',
      dimension: this.dimension,
      count: vectors.length,
      data,
      codebooks: this.codebooks,
      numSubvectors: this.numSubvectors,
      subvectorDim: this.subvectorDim,
    };
  }

  /**
   * Decode quantized vectors back to approximate floating-point representation
   *
   * @param quantized - Quantized vector container
   * @returns Array of reconstructed vectors
   */
  decode(quantized: QuantizedVectors): Float32Array[] {
    const { dimension, count, data, codebooks, numSubvectors, subvectorDim } = quantized;

    if (!codebooks || numSubvectors === undefined || subvectorDim === undefined) {
      throw new Error('Product quantized vectors require codebooks');
    }

    const vectors: Float32Array[] = new Array(count);

    for (let i = 0; i < count; i++) {
      const vec = new Float32Array(dimension);

      for (let m = 0; m < numSubvectors; m++) {
        const centroidIdx = data[i * numSubvectors + m];
        const codebook = codebooks[m];
        const centroid = codebook.subarray(
          centroidIdx * subvectorDim,
          (centroidIdx + 1) * subvectorDim
        );

        vec.set(centroid, m * subvectorDim);
      }

      vectors[i] = vec;
    }

    return vectors;
  }

  /**
   * Compute squared Euclidean distance between two vectors
   */
  private squaredDistance(a: Float32Array, b: Float32Array): number {
    let sum = 0;
    for (let i = 0; i < a.length; i++) {
      const diff = a[i] - b[i];
      sum += diff * diff;
    }
    return sum;
  }

  /**
   * Check if the model has been trained
   */
  isTrained(): boolean {
    return this.trained;
  }

  /**
   * Get codebooks for serialization
   */
  getCodebooks(): Float32Array[] {
    return this.codebooks;
  }

  /**
   * Load pre-trained codebooks
   *
   * @param codebooks - Array of codebook Float32Arrays
   * @param dimension - Original vector dimension
   */
  loadCodebooks(codebooks: Float32Array[], dimension: number): void {
    this.codebooks = codebooks;
    this.dimension = dimension;
    this.subvectorDim = dimension / this.numSubvectors;
    this.trained = true;
  }
}

// ============================================================================
// Quantized Vector Store
// ============================================================================

/**
 * A vector store that maintains vectors in quantized form for memory efficiency.
 * Uses asymmetric distance computation for improved accuracy during search.
 *
 * @example
 * ```typescript
 * const store = new QuantizedVectorStore({ type: 'scalar-8bit' });
 * store.add('vec1', new Float32Array([0.1, 0.2, 0.3]));
 * const results = store.search(new Float32Array([0.1, 0.2, 0.3]), 5);
 * ```
 */
export class QuantizedVectorStore {
  /** Quantization configuration */
  private readonly config: Required<QuantizationConfig>;
  /** Scalar quantization instance */
  private scalarQuantizer: ScalarQuantization | null = null;
  /** Product quantization instance */
  private productQuantizer: ProductQuantization | null = null;
  /** ID to index mapping */
  private idToIndex: Map<string, number> = new Map();
  /** Index to ID mapping */
  private indexToId: string[] = [];
  /** Raw vectors buffer (before quantization) */
  private rawVectors: Float32Array[] = [];
  /** Quantized vectors */
  private quantizedData: QuantizedVectors | null = null;
  /** Whether the store needs reindexing */
  private dirty = false;
  /** Batch size for auto-reindexing */
  private readonly batchSize = 1000;

  /**
   * Create a quantized vector store
   *
   * @param config - Quantization configuration
   */
  constructor(config: QuantizationConfig) {
    this.config = {
      type: config.type,
      numSubvectors: config.numSubvectors ?? 8,
      numCentroids: config.numCentroids ?? 256,
      trainingIterations: config.trainingIterations ?? 25,
    };

    if (config.type === 'scalar-4bit' || config.type === 'scalar-8bit') {
      this.scalarQuantizer = new ScalarQuantization();
    } else if (config.type === 'product') {
      this.productQuantizer = new ProductQuantization(
        this.config.numSubvectors,
        this.config.numCentroids,
        this.config.trainingIterations
      );
    }
  }

  /**
   * Add a vector to the store
   *
   * @param id - Unique identifier for the vector
   * @param vector - The vector to add
   */
  add(id: string, vector: Float32Array): void {
    if (this.idToIndex.has(id)) {
      // Update existing vector
      const index = this.idToIndex.get(id)!;
      this.rawVectors[index] = vector;
    } else {
      // Add new vector
      const index = this.rawVectors.length;
      this.idToIndex.set(id, index);
      this.indexToId.push(id);
      this.rawVectors.push(vector);
    }

    this.dirty = true;

    // Auto-reindex periodically for large stores
    if (this.rawVectors.length % this.batchSize === 0) {
      this.reindex();
    }
  }

  /**
   * Add multiple vectors in batch
   *
   * @param entries - Array of [id, vector] pairs
   */
  addBatch(entries: Array<[string, Float32Array]>): void {
    for (const [id, vector] of entries) {
      if (this.idToIndex.has(id)) {
        const index = this.idToIndex.get(id)!;
        this.rawVectors[index] = vector;
      } else {
        const index = this.rawVectors.length;
        this.idToIndex.set(id, index);
        this.indexToId.push(id);
        this.rawVectors.push(vector);
      }
    }

    this.dirty = true;
    this.reindex();
  }

  /**
   * Remove a vector from the store
   *
   * @param id - ID of vector to remove
   * @returns True if vector was removed
   */
  remove(id: string): boolean {
    const index = this.idToIndex.get(id);
    if (index === undefined) return false;

    // Swap with last element for O(1) removal
    const lastIndex = this.rawVectors.length - 1;
    if (index !== lastIndex) {
      const lastId = this.indexToId[lastIndex];
      this.rawVectors[index] = this.rawVectors[lastIndex];
      this.indexToId[index] = lastId;
      this.idToIndex.set(lastId, index);
    }

    this.rawVectors.pop();
    this.indexToId.pop();
    this.idToIndex.delete(id);
    this.dirty = true;

    return true;
  }

  /**
   * Force reindexing of quantized data
   */
  reindex(): void {
    if (this.rawVectors.length === 0) {
      this.quantizedData = null;
      this.dirty = false;
      return;
    }

    if (this.config.type === 'product') {
      // Train product quantizer if needed
      if (!this.productQuantizer!.isTrained()) {
        this.productQuantizer!.train(this.rawVectors);
      }
      this.quantizedData = this.productQuantizer!.encode(this.rawVectors);
    } else {
      // Scalar quantization
      const bits = this.config.type === 'scalar-8bit' ? 8 : 4;
      this.quantizedData = this.scalarQuantizer!.quantize(this.rawVectors, bits);
    }

    this.dirty = false;
  }

  /**
   * Search for nearest neighbors using asymmetric distance computation
   *
   * @param query - Query vector (full precision)
   * @param k - Number of neighbors to return
   * @returns Array of search results sorted by distance
   */
  search(query: Float32Array, k: number): SearchResult[] {
    if (this.rawVectors.length === 0) {
      return [];
    }

    // Ensure quantized data is up to date
    if (this.dirty || !this.quantizedData) {
      this.reindex();
    }

    const results: SearchResult[] = [];
    const n = Math.min(k, this.rawVectors.length);

    if (this.config.type === 'product') {
      // Precompute distance tables for asymmetric distance computation (ADC)
      const distanceTables = this.computeDistanceTables(query);

      for (let i = 0; i < this.quantizedData!.count; i++) {
        const distance = this.computeADCDistance(i, distanceTables);
        results.push({
          id: this.indexToId[i],
          distance,
          index: i,
        });
      }
    } else {
      // Scalar quantization with direct distance computation
      for (let i = 0; i < this.quantizedData!.count; i++) {
        const distance = this.scalarQuantizer!.computeDistance(this.quantizedData!, i, query);
        results.push({
          id: this.indexToId[i],
          distance,
          index: i,
        });
      }
    }

    // Sort by distance and return top k
    results.sort((a, b) => a.distance - b.distance);
    return results.slice(0, n);
  }

  /**
   * Precompute distance tables for asymmetric distance computation
   * This enables efficient lookup during search
   *
   * @param query - Query vector
   * @returns Distance lookup tables
   */
  private computeDistanceTables(query: Float32Array): Float32Array[] {
    const { numSubvectors, subvectorDim, codebooks } = this.quantizedData!;

    if (!codebooks || numSubvectors === undefined || subvectorDim === undefined) {
      throw new Error('Invalid product quantized data');
    }

    const tables: Float32Array[] = new Array(numSubvectors);

    for (let m = 0; m < numSubvectors; m++) {
      const table = new Float32Array(this.config.numCentroids);
      const querySubvector = query.slice(m * subvectorDim, (m + 1) * subvectorDim);
      const codebook = codebooks[m];

      for (let c = 0; c < this.config.numCentroids; c++) {
        const centroid = codebook.subarray(c * subvectorDim, (c + 1) * subvectorDim);
        let distSq = 0;

        for (let d = 0; d < subvectorDim; d++) {
          const diff = querySubvector[d] - centroid[d];
          distSq += diff * diff;
        }

        table[c] = distSq;
      }

      tables[m] = table;
    }

    return tables;
  }

  /**
   * Compute asymmetric distance using precomputed tables
   *
   * @param index - Vector index
   * @param tables - Precomputed distance tables
   * @returns Squared distance
   */
  private computeADCDistance(index: number, tables: Float32Array[]): number {
    const { data, numSubvectors } = this.quantizedData!;
    let totalDist = 0;

    for (let m = 0; m < numSubvectors!; m++) {
      const centroidIdx = data[index * numSubvectors! + m];
      totalDist += tables[m][centroidIdx];
    }

    return totalDist;
  }

  /**
   * Get a vector by ID (reconstructed from quantized form)
   *
   * @param id - Vector ID
   * @returns Reconstructed vector or null if not found
   */
  get(id: string): Float32Array | null {
    const index = this.idToIndex.get(id);
    if (index === undefined) return null;

    // Return raw vector for accuracy
    return this.rawVectors[index];
  }

  /**
   * Check if a vector exists in the store
   *
   * @param id - Vector ID
   * @returns True if vector exists
   */
  has(id: string): boolean {
    return this.idToIndex.has(id);
  }

  /**
   * Get the number of vectors in the store
   */
  get size(): number {
    return this.rawVectors.length;
  }

  /**
   * Get memory usage statistics
   */
  getMemoryStats(): {
    rawBytes: number;
    quantizedBytes: number;
    compressionRatio: number;
  } {
    const rawBytes = this.rawVectors.reduce((sum, v) => sum + v.byteLength, 0);
    const quantizedBytes = this.quantizedData?.data.byteLength ?? 0;
    const compressionRatio = quantizedBytes > 0 ? rawBytes / quantizedBytes : 0;

    return {
      rawBytes,
      quantizedBytes,
      compressionRatio,
    };
  }

  /**
   * Clear all vectors from the store
   */
  clear(): void {
    this.idToIndex.clear();
    this.indexToId = [];
    this.rawVectors = [];
    this.quantizedData = null;
    this.dirty = false;
  }

  /**
   * Export the store for serialization
   */
  export(): {
    config: Required<QuantizationConfig>;
    ids: string[];
    quantized: QuantizedVectors | null;
    codebooks?: Float32Array[];
  } {
    if (this.dirty) {
      this.reindex();
    }

    return {
      config: this.config,
      ids: [...this.indexToId],
      quantized: this.quantizedData,
      codebooks: this.productQuantizer?.getCodebooks(),
    };
  }
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Calculate the theoretical compression ratio for a quantization type
 *
 * @param type - Quantization type
 * @param dimension - Vector dimension
 * @param numSubvectors - Number of subvectors for product quantization
 * @returns Compression ratio (original size / compressed size)
 */
export function calculateCompressionRatio(
  type: QuantizationType,
  dimension: number,
  numSubvectors = 8
): number {
  const originalBytes = dimension * 4; // Float32 = 4 bytes

  switch (type) {
    case 'scalar-8bit':
      return originalBytes / dimension; // 1 byte per value = 4x compression
    case 'scalar-4bit':
      return originalBytes / Math.ceil(dimension / 2); // 0.5 bytes per value = 8x compression
    case 'product':
      return originalBytes / numSubvectors; // 1 byte per subvector
    default:
      return 1;
  }
}

/**
 * Estimate memory savings for a given number of vectors
 *
 * @param type - Quantization type
 * @param dimension - Vector dimension
 * @param numVectors - Number of vectors
 * @param numSubvectors - Number of subvectors for product quantization
 * @returns Memory savings in bytes
 */
export function estimateMemorySavings(
  type: QuantizationType,
  dimension: number,
  numVectors: number,
  numSubvectors = 8
): { originalBytes: number; compressedBytes: number; savedBytes: number; savedPercentage: number } {
  const originalBytes = dimension * 4 * numVectors;
  const ratio = calculateCompressionRatio(type, dimension, numSubvectors);
  const compressedBytes = Math.ceil(originalBytes / ratio);
  const savedBytes = originalBytes - compressedBytes;

  return {
    originalBytes,
    compressedBytes,
    savedBytes,
    savedPercentage: (savedBytes / originalBytes) * 100,
  };
}
