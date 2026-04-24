/**
 * Vector Quantization for AgentDB
 *
 * Provides memory-efficient vector storage through quantization techniques:
 *
 * 1. Scalar Quantization (8-bit and 4-bit)
 *    - 8-bit: 4x memory reduction (Float32 -> Uint8)
 *    - 4-bit: 8x memory reduction (Float32 -> 4-bit packed)
 *
 * 2. Product Quantization
 *    - Configurable subspaces and centroids
 *    - Higher compression ratios (up to 32x+)
 *    - Requires training on representative data
 *
 * 3. QuantizedVectorStore
 *    - Wraps vector collections with quantized storage
 *    - Asymmetric distance computation (full precision queries)
 *    - On-the-fly dequantization for similarity calculations
 */
// ============================================================================
// Performance & Security Constants
// ============================================================================
/** @inline Maximum allowed vector dimension to prevent DoS via large allocations */
export const MAX_VECTOR_DIMENSION = 4096;
/** @inline Maximum number of vectors in a store */
const MAX_STORE_SIZE = 10_000_000;
/** @inline Maximum k-means iterations to prevent CPU exhaustion */
const MAX_KMEANS_ITERATIONS = 500;
/** @inline Maximum training vectors for product quantization */
const MAX_TRAINING_VECTORS = 1_000_000;
/** @inline Maximum batch size for optimal cache utilization */
export const MAX_BATCH_SIZE = 10000;
/** @inline Default cache size for embedding storage */
export const DEFAULT_CACHE_SIZE = 10000;
/** @inline Pre-computed inverse of 255 for 8-bit dequantization */
const INV_255 = 1 / 255;
/** @inline Pre-computed inverse of 15 for 4-bit dequantization */
const INV_15 = 1 / 15;
/** @inline Small epsilon for numerical stability */
const EPSILON = 1e-10;
// ============================================================================
// Input Validation Helpers
// ============================================================================
/**
 * Validate Float32Array input
 */
function validateFloat32Array(arr, name) {
    if (arr === null || arr === undefined) {
        throw new Error(`${name} cannot be null or undefined`);
    }
    if (!(arr instanceof Float32Array)) {
        throw new Error(`${name} must be a Float32Array`);
    }
}
/**
 * Validate dimension is within safe limits
 */
function validateDimension(dimension, name = 'dimension') {
    if (!Number.isFinite(dimension) || dimension < 0) {
        throw new Error(`${name} must be a non-negative finite number`);
    }
    if (dimension > MAX_VECTOR_DIMENSION) {
        throw new Error(`${name} exceeds maximum allowed size of ${MAX_VECTOR_DIMENSION}`);
    }
}
/**
 * Safely parse JSON with validation
 */
function safeJSONParse(json, validator) {
    let parsed;
    try {
        parsed = JSON.parse(json);
    }
    catch (e) {
        throw new Error('Invalid JSON format');
    }
    if (!validator(parsed)) {
        throw new Error('JSON structure does not match expected format');
    }
    return parsed;
}
// ============================================================================
// Scalar Quantization Functions
// ============================================================================
/**
 * Quantize a vector to 8-bit values (4x memory reduction)
 *
 * Formula: quantized = round((value - min) / (max - min) * 255)
 *
 * @param vector - Input vector as Float32Array
 * @returns Object containing quantized data and min/max for dequantization
 */
export function quantize8bit(vector) {
    // Input validation
    validateFloat32Array(vector, 'Input vector');
    validateDimension(vector.length, 'Vector dimension');
    if (vector.length === 0) {
        return {
            data: new Uint8Array(0),
            min: 0,
            max: 0,
            dimension: 0,
            type: '8bit',
        };
    }
    // Find min and max values
    let min = vector[0];
    let max = vector[0];
    for (let i = 1; i < vector.length; i++) {
        if (vector[i] < min)
            min = vector[i];
        if (vector[i] > max)
            max = vector[i];
    }
    // Handle edge case where all values are the same
    const range = max - min;
    const scale = range === 0 ? 0 : 255 / range;
    // Quantize
    const quantized = new Uint8Array(vector.length);
    for (let i = 0; i < vector.length; i++) {
        quantized[i] = Math.round((vector[i] - min) * scale);
    }
    return {
        data: quantized,
        min,
        max,
        dimension: vector.length,
        type: '8bit',
    };
}
/**
 * Quantize a vector to 4-bit values (8x memory reduction)
 *
 * Formula: quantized = round((value - min) / (max - min) * 15)
 * Two 4-bit values are packed into each byte.
 *
 * @param vector - Input vector as Float32Array
 * @returns Object containing quantized data (packed) and min/max for dequantization
 */
export function quantize4bit(vector) {
    // Input validation
    validateFloat32Array(vector, 'Input vector');
    validateDimension(vector.length, 'Vector dimension');
    if (vector.length === 0) {
        return {
            data: new Uint8Array(0),
            min: 0,
            max: 0,
            dimension: 0,
            type: '4bit',
        };
    }
    // Find min and max values
    let min = vector[0];
    let max = vector[0];
    for (let i = 1; i < vector.length; i++) {
        if (vector[i] < min)
            min = vector[i];
        if (vector[i] > max)
            max = vector[i];
    }
    // Handle edge case where all values are the same
    const range = max - min;
    const scale = range === 0 ? 0 : 15 / range;
    // Quantize and pack two values per byte
    const packedLength = Math.ceil(vector.length / 2);
    const quantized = new Uint8Array(packedLength);
    for (let i = 0; i < vector.length; i += 2) {
        const high = Math.round((vector[i] - min) * scale) & 0x0f;
        const low = i + 1 < vector.length
            ? Math.round((vector[i + 1] - min) * scale) & 0x0f
            : 0;
        quantized[i >> 1] = (high << 4) | low;
    }
    return {
        data: quantized,
        min,
        max,
        dimension: vector.length,
        type: '4bit',
    };
}
/**
 * Dequantize an 8-bit vector back to Float32
 *
 * @param quantized - Quantized data as Uint8Array
 * @param min - Minimum value from original quantization
 * @param max - Maximum value from original quantization
 * @returns Reconstructed Float32Array
 */
export function dequantize8bit(quantized, min, max) {
    if (quantized.length === 0) {
        return new Float32Array(0);
    }
    const range = max - min;
    // @inline Use pre-computed constant when possible, fallback to computed scale
    const scale = range === 0 ? 0 : range * INV_255;
    const len = quantized.length | 0; // Force integer
    const result = new Float32Array(len);
    // @inline 4x loop unrolling for better ILP
    const loopEnd = (len - (len & 3)) | 0;
    for (let i = 0; i < loopEnd; i = (i + 4) | 0) {
        result[i] = quantized[i] * scale + min;
        result[i + 1] = quantized[i + 1] * scale + min;
        result[i + 2] = quantized[i + 2] * scale + min;
        result[i + 3] = quantized[i + 3] * scale + min;
    }
    // Handle remainder
    for (let i = loopEnd; i < len; i++) {
        result[i] = quantized[i] * scale + min;
    }
    return result;
}
/**
 * Dequantize a 4-bit packed vector back to Float32
 *
 * @param quantized - Packed quantized data as Uint8Array
 * @param min - Minimum value from original quantization
 * @param max - Maximum value from original quantization
 * @param originalLength - Original vector length (needed due to packing)
 * @returns Reconstructed Float32Array
 */
export function dequantize4bit(quantized, min, max, originalLength) {
    if (quantized.length === 0) {
        return new Float32Array(0);
    }
    // Validate min/max are valid numbers
    if (!Number.isFinite(min) || !Number.isFinite(max)) {
        throw new Error('min and max must be finite numbers');
    }
    const range = max - min;
    const scale = range === 0 ? 0 : range / 15;
    // Calculate output length with bounds checking
    const maxPossibleLength = quantized.length * 2;
    const length = originalLength ?? maxPossibleLength;
    // Security check: originalLength should not exceed what the data can provide
    if (length > maxPossibleLength) {
        throw new Error(`originalLength (${length}) exceeds maximum possible from data (${maxPossibleLength})`);
    }
    validateDimension(length, 'Output length');
    const result = new Float32Array(length);
    for (let i = 0; i < length; i++) {
        const byteIdx = i >> 1;
        // Bounds check
        if (byteIdx >= quantized.length) {
            break;
        }
        const isHigh = (i & 1) === 0;
        const nibble = isHigh
            ? (quantized[byteIdx] >> 4) & 0x0f
            : quantized[byteIdx] & 0x0f;
        result[i] = nibble * scale + min;
    }
    return result;
}
/**
 * Calculate quantization error statistics
 *
 * @param original - Original vector
 * @param reconstructed - Reconstructed vector after quantization/dequantization
 * @returns Statistics about the quantization error
 */
export function calculateQuantizationError(original, reconstructed) {
    if (original.length !== reconstructed.length) {
        throw new Error('Vector lengths must match');
    }
    if (original.length === 0) {
        return { meanError: 0, maxError: 0, mse: 0 };
    }
    let sumError = 0;
    let sumSquaredError = 0;
    let maxError = 0;
    for (let i = 0; i < original.length; i++) {
        const error = Math.abs(original[i] - reconstructed[i]);
        sumError += error;
        sumSquaredError += error * error;
        if (error > maxError)
            maxError = error;
    }
    return {
        meanError: sumError / original.length,
        maxError,
        mse: sumSquaredError / original.length,
    };
}
/**
 * Get quantization statistics for a vector
 *
 * @param vector - Original vector
 * @param type - Quantization type ('8bit' or '4bit')
 * @returns Quantization statistics including error metrics
 */
export function getQuantizationStats(vector, type) {
    const quantized = type === '8bit' ? quantize8bit(vector) : quantize4bit(vector);
    const reconstructed = type === '8bit'
        ? dequantize8bit(quantized.data, quantized.min, quantized.max)
        : dequantize4bit(quantized.data, quantized.min, quantized.max, quantized.dimension);
    const errors = calculateQuantizationError(vector, reconstructed);
    // Calculate compression ratio
    // Original: vector.length * 4 bytes (Float32)
    // 8-bit: vector.length bytes
    // 4-bit: ceil(vector.length / 2) bytes
    const originalBytes = vector.length * 4;
    const compressedBytes = type === '8bit' ? quantized.data.length : Math.ceil(vector.length / 2);
    const compressionRatio = originalBytes / compressedBytes;
    return {
        min: quantized.min,
        max: quantized.max,
        meanError: errors.meanError,
        maxError: errors.maxError,
        compressionRatio,
    };
}
// ============================================================================
// Product Quantization
// ============================================================================
/**
 * Simple seeded random number generator (LCG)
 */
class SeededRandom {
    seed;
    constructor(seed) {
        this.seed = seed;
    }
    next() {
        this.seed = (this.seed * 1103515245 + 12345) & 0x7fffffff;
        return this.seed / 0x7fffffff;
    }
    nextInt(max) {
        return Math.floor(this.next() * max);
    }
}
/**
 * Product Quantizer for high compression ratios
 *
 * Divides vectors into subspaces and learns centroids for each subspace.
 * Vectors are encoded as indices into the centroid codebooks.
 */
export class ProductQuantizer {
    config;
    subspaceDim;
    codebooks = null;
    trained = false;
    rng;
    constructor(config) {
        // Validate configuration
        validateDimension(config.dimension, 'Dimension');
        if (config.dimension % config.numSubspaces !== 0) {
            throw new Error(`Dimension (${config.dimension}) must be divisible by numSubspaces (${config.numSubspaces})`);
        }
        if (config.numCentroids > 256) {
            throw new Error('numCentroids must be <= 256 for uint8 encoding');
        }
        if (config.numCentroids < 2) {
            throw new Error('numCentroids must be >= 2');
        }
        if (config.numSubspaces < 1 || config.numSubspaces > 256) {
            throw new Error('numSubspaces must be between 1 and 256');
        }
        // Limit max iterations to prevent CPU exhaustion
        const maxIterations = Math.min(config.maxIterations ?? 50, MAX_KMEANS_ITERATIONS);
        this.config = {
            dimension: config.dimension,
            numSubspaces: config.numSubspaces,
            numCentroids: config.numCentroids,
            maxIterations: maxIterations,
            convergenceThreshold: config.convergenceThreshold ?? 1e-4,
            seed: config.seed ?? Date.now(),
        };
        this.subspaceDim = this.config.dimension / this.config.numSubspaces;
        this.rng = new SeededRandom(this.config.seed);
    }
    /**
     * Train the codebooks using k-means clustering
     *
     * @param vectors - Training vectors (should be representative of the data)
     */
    async train(vectors) {
        if (vectors.length === 0) {
            throw new Error('Training requires at least one vector');
        }
        // Limit training vectors to prevent memory exhaustion
        if (vectors.length > MAX_TRAINING_VECTORS) {
            throw new Error(`Training vectors exceed maximum of ${MAX_TRAINING_VECTORS}`);
        }
        if (vectors.length < this.config.numCentroids) {
            throw new Error(`Need at least ${this.config.numCentroids} vectors to train ${this.config.numCentroids} centroids`);
        }
        // Validate vector dimensions
        for (const vec of vectors) {
            validateFloat32Array(vec, 'Training vector');
            if (vec.length !== this.config.dimension) {
                throw new Error(`Vector dimension (${vec.length}) does not match config (${this.config.dimension})`);
            }
        }
        this.codebooks = [];
        // Train each subspace independently
        for (let s = 0; s < this.config.numSubspaces; s++) {
            const startDim = s * this.subspaceDim;
            // Extract subvectors for this subspace
            const subvectors = vectors.map((v) => v.slice(startDim, startDim + this.subspaceDim));
            // Run k-means to find centroids
            const centroids = await this.kMeans(subvectors);
            this.codebooks.push(centroids);
        }
        this.trained = true;
    }
    /**
     * K-means clustering with k-means++ initialization
     */
    async kMeans(subvectors) {
        const n = subvectors.length;
        const k = this.config.numCentroids;
        // K-means++ initialization
        const centroids = this.kMeansPlusPlus(subvectors, k);
        const assignments = new Uint32Array(n);
        let prevInertia = Infinity;
        for (let iter = 0; iter < this.config.maxIterations; iter++) {
            // Assign each vector to nearest centroid
            let inertia = 0;
            for (let i = 0; i < n; i++) {
                let minDist = Infinity;
                let minIdx = 0;
                for (let c = 0; c < k; c++) {
                    const dist = this.squaredL2Distance(subvectors[i], centroids[c]);
                    if (dist < minDist) {
                        minDist = dist;
                        minIdx = c;
                    }
                }
                assignments[i] = minIdx;
                inertia += minDist;
            }
            // Check convergence
            if (Math.abs(prevInertia - inertia) < this.config.convergenceThreshold) {
                break;
            }
            prevInertia = inertia;
            // Update centroids
            const counts = new Uint32Array(k);
            const sums = Array.from({ length: k }, () => new Float32Array(this.subspaceDim));
            for (let i = 0; i < n; i++) {
                const cluster = assignments[i];
                counts[cluster]++;
                for (let d = 0; d < this.subspaceDim; d++) {
                    sums[cluster][d] += subvectors[i][d];
                }
            }
            for (let c = 0; c < k; c++) {
                if (counts[c] > 0) {
                    for (let d = 0; d < this.subspaceDim; d++) {
                        centroids[c][d] = sums[c][d] / counts[c];
                    }
                }
            }
            // Yield to event loop periodically
            if (iter % 10 === 0) {
                await new Promise((resolve) => setTimeout(resolve, 0));
            }
        }
        return centroids;
    }
    /**
     * K-means++ initialization for better centroid selection
     */
    kMeansPlusPlus(vectors, k) {
        const n = vectors.length;
        const centroids = [];
        // Choose first centroid randomly
        const firstIdx = this.rng.nextInt(n);
        centroids.push(new Float32Array(vectors[firstIdx]));
        // Choose remaining centroids with probability proportional to distance squared
        for (let i = 1; i < k; i++) {
            const distances = new Float32Array(n);
            let sumDistances = 0;
            for (let j = 0; j < n; j++) {
                let minDist = Infinity;
                for (const centroid of centroids) {
                    const dist = this.squaredL2Distance(vectors[j], centroid);
                    minDist = Math.min(minDist, dist);
                }
                distances[j] = minDist;
                sumDistances += minDist;
            }
            // Sample next centroid
            let r = this.rng.next() * sumDistances;
            for (let j = 0; j < n; j++) {
                r -= distances[j];
                if (r <= 0) {
                    centroids.push(new Float32Array(vectors[j]));
                    break;
                }
            }
            // Fallback if we didn't select (numerical issues)
            if (centroids.length === i) {
                centroids.push(new Float32Array(vectors[this.rng.nextInt(n)]));
            }
        }
        return centroids;
    }
    /**
     * Encode a vector using the trained codebooks
     *
     * @param vector - Vector to encode
     * @returns Encoded vector with centroid codes and norm
     */
    encode(vector) {
        if (!this.trained || !this.codebooks) {
            throw new Error('ProductQuantizer must be trained before encoding');
        }
        if (vector.length !== this.config.dimension) {
            throw new Error(`Vector dimension (${vector.length}) does not match config (${this.config.dimension})`);
        }
        const codes = new Uint8Array(this.config.numSubspaces);
        // Calculate norm for potential normalization
        let norm = 0;
        for (let i = 0; i < vector.length; i++) {
            norm += vector[i] * vector[i];
        }
        norm = Math.sqrt(norm);
        // Encode each subspace
        for (let s = 0; s < this.config.numSubspaces; s++) {
            const startDim = s * this.subspaceDim;
            const subvector = vector.slice(startDim, startDim + this.subspaceDim);
            // Find nearest centroid
            let minDist = Infinity;
            let minIdx = 0;
            for (let c = 0; c < this.config.numCentroids; c++) {
                const dist = this.squaredL2Distance(subvector, this.codebooks[s][c]);
                if (dist < minDist) {
                    minDist = dist;
                    minIdx = c;
                }
            }
            codes[s] = minIdx;
        }
        return { codes, norm };
    }
    /**
     * Decode an encoded vector (approximate reconstruction)
     *
     * @param encoded - Encoded vector
     * @returns Reconstructed Float32Array
     */
    decode(encoded) {
        if (!this.codebooks) {
            throw new Error('ProductQuantizer must be trained before decoding');
        }
        const vector = new Float32Array(this.config.dimension);
        for (let s = 0; s < this.config.numSubspaces; s++) {
            const code = encoded.codes[s];
            const centroid = this.codebooks[s][code];
            const startDim = s * this.subspaceDim;
            for (let d = 0; d < this.subspaceDim; d++) {
                vector[startDim + d] = centroid[d];
            }
        }
        return vector;
    }
    /**
     * Compute asymmetric distance between a query and encoded vector
     * Query is in full precision, database vector is encoded
     *
     * @param query - Full precision query vector
     * @param encoded - Encoded database vector
     * @returns Squared L2 distance
     */
    asymmetricDistance(query, encoded) {
        if (!this.codebooks) {
            throw new Error('ProductQuantizer must be trained');
        }
        let distance = 0;
        for (let s = 0; s < this.config.numSubspaces; s++) {
            const startDim = s * this.subspaceDim;
            const querySubvector = query.slice(startDim, startDim + this.subspaceDim);
            const centroid = this.codebooks[s][encoded.codes[s]];
            distance += this.squaredL2Distance(querySubvector, centroid);
        }
        return distance;
    }
    /**
     * Precompute distance tables for efficient batch search
     * Tables[s][c] = squared distance from query subvector s to centroid c
     *
     * @param query - Query vector
     * @returns Distance lookup tables for each subspace
     */
    precomputeDistanceTables(query) {
        if (!this.codebooks) {
            throw new Error('ProductQuantizer must be trained');
        }
        const tables = [];
        for (let s = 0; s < this.config.numSubspaces; s++) {
            const table = new Float32Array(this.config.numCentroids);
            const startDim = s * this.subspaceDim;
            const querySubvector = query.slice(startDim, startDim + this.subspaceDim);
            for (let c = 0; c < this.config.numCentroids; c++) {
                table[c] = this.squaredL2Distance(querySubvector, this.codebooks[s][c]);
            }
            tables.push(table);
        }
        return tables;
    }
    /**
     * Compute distance using precomputed tables (very fast)
     *
     * @param tables - Precomputed distance tables from precomputeDistanceTables()
     * @param encoded - Encoded vector
     * @returns Squared L2 distance
     */
    distanceFromTables(tables, encoded) {
        let distance = 0;
        for (let s = 0; s < this.config.numSubspaces; s++) {
            distance += tables[s][encoded.codes[s]];
        }
        return distance;
    }
    /**
     * Get compression ratio
     */
    getCompressionRatio() {
        // Original: dimension * 4 bytes (Float32)
        // Encoded: numSubspaces * 1 byte (Uint8) + 4 bytes (norm)
        const originalBytes = this.config.dimension * 4;
        const encodedBytes = this.config.numSubspaces + 4;
        return originalBytes / encodedBytes;
    }
    /**
     * Get quantizer statistics
     */
    getStats() {
        const codebookSize = this.codebooks
            ? this.config.numSubspaces *
                this.config.numCentroids *
                this.subspaceDim *
                4
            : 0;
        return {
            trained: this.trained,
            dimension: this.config.dimension,
            numSubspaces: this.config.numSubspaces,
            subspaceDim: this.subspaceDim,
            numCentroids: this.config.numCentroids,
            compressionRatio: this.getCompressionRatio(),
            codebookSizeBytes: codebookSize,
        };
    }
    /**
     * Export codebooks for persistence
     */
    exportCodebooks() {
        if (!this.codebooks) {
            throw new Error('No codebooks to export');
        }
        return JSON.stringify({
            config: this.config,
            codebooks: this.codebooks.map((subspace) => subspace.map((centroid) => Array.from(centroid))),
        });
    }
    /**
     * Import codebooks from persisted data
     * Includes validation to prevent prototype pollution and type confusion
     */
    importCodebooks(json) {
        // Type guard for imported data
        function isValidCodebookData(obj) {
            if (typeof obj !== 'object' || obj === null)
                return false;
            const data = obj;
            // Validate config
            if (typeof data.config !== 'object' || data.config === null)
                return false;
            const config = data.config;
            if (typeof config.dimension !== 'number' || !Number.isFinite(config.dimension))
                return false;
            if (typeof config.numSubspaces !== 'number' || !Number.isFinite(config.numSubspaces))
                return false;
            if (typeof config.numCentroids !== 'number' || !Number.isFinite(config.numCentroids))
                return false;
            // Validate codebooks is array of arrays
            if (!Array.isArray(data.codebooks))
                return false;
            for (const subspace of data.codebooks) {
                if (!Array.isArray(subspace))
                    return false;
                for (const centroid of subspace) {
                    if (!Array.isArray(centroid))
                        return false;
                    for (const val of centroid) {
                        if (typeof val !== 'number')
                            return false;
                    }
                }
            }
            return true;
        }
        const data = safeJSONParse(json, isValidCodebookData);
        // Validate imported config
        validateDimension(data.config.dimension, 'Imported dimension');
        if (data.config.numCentroids > 256 || data.config.numCentroids < 2) {
            throw new Error('Invalid numCentroids in imported data');
        }
        if (data.config.numSubspaces < 1 || data.config.numSubspaces > 256) {
            throw new Error('Invalid numSubspaces in imported data');
        }
        if (data.config.dimension % data.config.numSubspaces !== 0) {
            throw new Error('Dimension must be divisible by numSubspaces');
        }
        this.config = {
            ...data.config,
            maxIterations: Math.min(data.config.maxIterations ?? 50, MAX_KMEANS_ITERATIONS),
            convergenceThreshold: data.config.convergenceThreshold ?? 1e-4,
            seed: data.config.seed ?? Date.now(),
        };
        this.subspaceDim = this.config.dimension / this.config.numSubspaces;
        this.codebooks = data.codebooks.map((subspace) => subspace.map((centroid) => new Float32Array(centroid)));
        this.trained = true;
    }
    /**
     * Check if the quantizer is trained
     */
    isTrained() {
        return this.trained;
    }
    /**
     * Squared L2 distance utility
     */
    squaredL2Distance(a, b) {
        let sum = 0;
        for (let i = 0; i < a.length; i++) {
            const diff = a[i] - b[i];
            sum += diff * diff;
        }
        return sum;
    }
}
/**
 * QuantizedVectorStore - Memory-efficient vector storage
 *
 * Wraps vector collections with quantized storage while providing
 * full-precision query support through asymmetric distance computation.
 */
export class QuantizedVectorStore {
    config;
    vectors = new Map();
    productQuantizer = null;
    constructor(config) {
        this.config = {
            dimension: config.dimension,
            quantizationType: config.quantizationType,
            productQuantizerConfig: config.productQuantizerConfig ?? {
                numSubspaces: 8,
                numCentroids: 256,
            },
            metric: config.metric ?? 'cosine',
        };
        if (this.config.quantizationType === 'product') {
            this.productQuantizer = new ProductQuantizer({
                dimension: this.config.dimension,
                ...this.config.productQuantizerConfig,
            });
        }
    }
    /**
     * Train the product quantizer (required for 'product' quantization type)
     *
     * @param vectors - Training vectors
     */
    async train(vectors) {
        if (this.config.quantizationType !== 'product') {
            throw new Error('Training only required for product quantization');
        }
        if (!this.productQuantizer) {
            throw new Error('ProductQuantizer not initialized');
        }
        await this.productQuantizer.train(vectors);
    }
    /**
     * Check if the store is ready for insertions
     */
    isReady() {
        if (this.config.quantizationType === 'product') {
            return this.productQuantizer?.isTrained() ?? false;
        }
        return true;
    }
    /**
     * Insert a vector into the store
     *
     * @param id - Unique identifier
     * @param vector - Vector to store
     * @param metadata - Optional metadata
     */
    insert(id, vector, metadata) {
        // Validate inputs
        validateFloat32Array(vector, 'Vector');
        if (vector.length !== this.config.dimension) {
            throw new Error(`Vector dimension (${vector.length}) does not match store dimension (${this.config.dimension})`);
        }
        // Check store size limit
        if (this.vectors.size >= MAX_STORE_SIZE) {
            throw new Error(`Store has reached maximum capacity of ${MAX_STORE_SIZE} vectors`);
        }
        let quantized;
        switch (this.config.quantizationType) {
            case 'scalar8bit':
                quantized = quantize8bit(vector);
                break;
            case 'scalar4bit':
                quantized = quantize4bit(vector);
                break;
            case 'product':
                if (!this.productQuantizer?.isTrained()) {
                    throw new Error('ProductQuantizer must be trained before inserting vectors');
                }
                quantized = this.productQuantizer.encode(vector);
                break;
        }
        this.vectors.set(id, { id, quantized, metadata });
    }
    /**
     * Insert multiple vectors in batch
     *
     * @param items - Array of {id, vector, metadata}
     */
    insertBatch(items) {
        for (const item of items) {
            this.insert(item.id, item.vector, item.metadata);
        }
    }
    /**
     * Search for nearest neighbors
     * Uses asymmetric distance computation: query in full precision,
     * database vectors in quantized form.
     *
     * @param query - Query vector (full precision)
     * @param k - Number of results to return
     * @param threshold - Optional similarity threshold
     * @returns Sorted search results (most similar first)
     */
    search(query, k, threshold) {
        if (query.length !== this.config.dimension) {
            throw new Error(`Query dimension (${query.length}) does not match store dimension (${this.config.dimension})`);
        }
        if (this.vectors.size === 0) {
            return [];
        }
        // Normalize query for cosine similarity
        const normalizedQuery = this.config.metric === 'cosine' ? this.normalize(query) : query;
        // Precompute distance tables for product quantization
        let distanceTables = null;
        if (this.config.quantizationType === 'product' &&
            this.productQuantizer?.isTrained()) {
            distanceTables = this.productQuantizer.precomputeDistanceTables(normalizedQuery);
        }
        // Calculate distances to all vectors
        const results = [];
        for (const stored of this.vectors.values()) {
            const distance = this.computeAsymmetricDistance(normalizedQuery, stored.quantized, distanceTables);
            results.push({ id: stored.id, distance, metadata: stored.metadata });
        }
        // Sort by distance (ascending for L2, we'll convert to similarity later)
        results.sort((a, b) => a.distance - b.distance);
        // Convert to search results with similarity scores
        const maxDistance = Math.max(...results.map((r) => r.distance), 1e-10);
        const searchResults = [];
        for (let i = 0; i < Math.min(k, results.length); i++) {
            const r = results[i];
            // Convert distance to similarity
            let similarity;
            if (this.config.metric === 'cosine' || this.config.metric === 'ip') {
                // For cosine/IP, we stored squared distance but actually want similarity
                // Similarity = 1 - (distance / 2) for normalized vectors with L2
                similarity = Math.max(0, 1 - Math.sqrt(r.distance) / 2);
            }
            else {
                // For L2, use inverse distance normalization
                similarity = 1 / (1 + r.distance);
            }
            if (threshold !== undefined && similarity < threshold) {
                continue;
            }
            searchResults.push({
                id: r.id,
                distance: r.distance,
                similarity,
                metadata: r.metadata,
            });
        }
        return searchResults;
    }
    /**
     * Remove a vector by ID
     *
     * @param id - Vector ID to remove
     * @returns true if removed, false if not found
     */
    remove(id) {
        return this.vectors.delete(id);
    }
    /**
     * Get a dequantized vector by ID
     *
     * @param id - Vector ID
     * @returns Reconstructed vector or null if not found
     */
    getVector(id) {
        const stored = this.vectors.get(id);
        if (!stored) {
            return null;
        }
        return this.dequantizeStored(stored.quantized);
    }
    /**
     * Get store statistics
     */
    getStats() {
        let bytesPerVector;
        let codebookBytes = 0;
        switch (this.config.quantizationType) {
            case 'scalar8bit':
                bytesPerVector = this.config.dimension + 8 + 4; // data + min/max + dimension
                break;
            case 'scalar4bit':
                bytesPerVector = Math.ceil(this.config.dimension / 2) + 8 + 4;
                break;
            case 'product':
                const pqStats = this.productQuantizer?.getStats();
                bytesPerVector = (pqStats?.numSubspaces ?? 8) + 4; // codes + norm
                codebookBytes = pqStats?.codebookSizeBytes ?? 0;
                break;
            default:
                bytesPerVector = this.config.dimension * 4;
        }
        const originalBytesPerVector = this.config.dimension * 4;
        const compressionRatio = originalBytesPerVector / bytesPerVector;
        return {
            count: this.vectors.size,
            dimension: this.config.dimension,
            quantizationType: this.config.quantizationType,
            metric: this.config.metric,
            compressionRatio,
            memoryUsageBytes: this.vectors.size * bytesPerVector + codebookBytes,
            productQuantizerStats: this.productQuantizer?.getStats(),
        };
    }
    /**
     * Clear all vectors
     */
    clear() {
        this.vectors.clear();
    }
    /**
     * Export store to JSON (for persistence)
     */
    export() {
        const data = {
            config: this.config,
            vectors: [],
        };
        for (const stored of this.vectors.values()) {
            const exportedQuantized = this.config.quantizationType === 'product'
                ? {
                    codes: Array.from(stored.quantized.codes),
                    norm: stored.quantized.norm,
                }
                : {
                    data: Array.from(stored.quantized.data),
                    min: stored.quantized.min,
                    max: stored.quantized.max,
                    dimension: stored.quantized.dimension,
                    type: stored.quantized.type,
                };
            data.vectors.push({
                id: stored.id,
                quantized: exportedQuantized,
                metadata: stored.metadata,
            });
        }
        if (this.productQuantizer?.isTrained()) {
            data.codebooks = this.productQuantizer.exportCodebooks();
        }
        return JSON.stringify(data);
    }
    /**
     * Import store from JSON
     */
    import(json) {
        const data = JSON.parse(json);
        this.config = data.config;
        this.vectors.clear();
        if (data.codebooks && this.config.quantizationType === 'product') {
            this.productQuantizer = new ProductQuantizer({
                dimension: this.config.dimension,
                ...this.config.productQuantizerConfig,
            });
            this.productQuantizer.importCodebooks(data.codebooks);
        }
        for (const stored of data.vectors) {
            const quantized = this.config.quantizationType === 'product'
                ? {
                    codes: new Uint8Array(stored.quantized.codes),
                    norm: stored.quantized.norm,
                }
                : {
                    data: new Uint8Array(stored.quantized.data),
                    min: stored.quantized.min,
                    max: stored.quantized.max,
                    dimension: stored.quantized.dimension,
                    type: stored.quantized.type,
                };
            this.vectors.set(stored.id, {
                id: stored.id,
                quantized: quantized,
                metadata: stored.metadata,
            });
        }
    }
    /**
     * Compute asymmetric distance (query full precision, database quantized)
     */
    computeAsymmetricDistance(query, quantized, distanceTables) {
        switch (this.config.quantizationType) {
            case 'scalar8bit':
            case 'scalar4bit': {
                const q = quantized;
                const dequantized = q.type === '8bit'
                    ? dequantize8bit(q.data, q.min, q.max)
                    : dequantize4bit(q.data, q.min, q.max, q.dimension);
                // Normalize for cosine
                const normalizedDeq = this.config.metric === 'cosine' ? this.normalize(dequantized) : dequantized;
                return this.squaredL2Distance(query, normalizedDeq);
            }
            case 'product': {
                if (distanceTables && this.productQuantizer) {
                    return this.productQuantizer.distanceFromTables(distanceTables, quantized);
                }
                if (this.productQuantizer) {
                    return this.productQuantizer.asymmetricDistance(query, quantized);
                }
                throw new Error('ProductQuantizer not initialized');
            }
        }
    }
    /**
     * Dequantize a stored vector
     */
    dequantizeStored(quantized) {
        if (this.config.quantizationType === 'product') {
            if (!this.productQuantizer) {
                throw new Error('ProductQuantizer not initialized');
            }
            return this.productQuantizer.decode(quantized);
        }
        const q = quantized;
        return q.type === '8bit'
            ? dequantize8bit(q.data, q.min, q.max)
            : dequantize4bit(q.data, q.min, q.max, q.dimension);
    }
    /**
     * Normalize a vector to unit length
     */
    normalize(vector) {
        let norm = 0;
        for (let i = 0; i < vector.length; i++) {
            norm += vector[i] * vector[i];
        }
        norm = Math.sqrt(norm);
        if (norm === 0) {
            return vector;
        }
        const result = new Float32Array(vector.length);
        for (let i = 0; i < vector.length; i++) {
            result[i] = vector[i] / norm;
        }
        return result;
    }
    /**
     * Squared L2 distance
     */
    squaredL2Distance(a, b) {
        let sum = 0;
        for (let i = 0; i < a.length; i++) {
            const diff = a[i] - b[i];
            sum += diff * diff;
        }
        return sum;
    }
}
// ============================================================================
// Factory Functions
// ============================================================================
/**
 * Create an 8-bit scalar quantized vector store (4x compression)
 */
export function createScalar8BitStore(dimension, metric = 'cosine') {
    return new QuantizedVectorStore({
        dimension,
        quantizationType: 'scalar8bit',
        metric,
    });
}
/**
 * Create a 4-bit scalar quantized vector store (8x compression)
 */
export function createScalar4BitStore(dimension, metric = 'cosine') {
    return new QuantizedVectorStore({
        dimension,
        quantizationType: 'scalar4bit',
        metric,
    });
}
/**
 * Create a product quantized vector store (configurable compression)
 *
 * @param dimension - Vector dimension
 * @param numSubspaces - Number of subspaces (8, 16, 32, 64 typical)
 * @param numCentroids - Number of centroids per subspace (256 max for uint8)
 * @param metric - Distance metric
 */
export function createProductQuantizedStore(dimension, numSubspaces = 8, numCentroids = 256, metric = 'cosine') {
    return new QuantizedVectorStore({
        dimension,
        quantizationType: 'product',
        productQuantizerConfig: {
            numSubspaces,
            numCentroids,
        },
        metric,
    });
}
//# sourceMappingURL=vector-quantization.js.map