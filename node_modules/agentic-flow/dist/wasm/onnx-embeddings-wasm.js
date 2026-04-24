/**
 * ONNX Embeddings WASM Integration
 *
 * Provides browser-compatible embeddings using ruvector-onnx-embeddings-wasm.
 *
 * Features:
 * - Pure WASM: No native dependencies, runs in browser
 * - all-MiniLM-L6-v2: 384-dimensional semantic embeddings
 * - SIMD Acceleration: Uses WebAssembly SIMD when available
 * - Small Bundle: Optimized WASM binary size
 *
 * Use Cases:
 * - Browser-based semantic search
 * - Client-side RAG applications
 * - Edge computing (Cloudflare Workers, Deno Deploy)
 * - Offline-first applications
 *
 * Performance:
 * - ~50ms per embedding (SIMD enabled)
 * - ~200ms per embedding (SIMD disabled)
 * - Batch processing: ~30ms per embedding
 */
import { logger } from '../utils/logger.js';
// Module state
let wasmModule = null;
let initialized = false;
let initPromise = null;
// Stats tracking
let stats = {
    totalEmbeddings: 0,
    totalLatencyMs: 0,
};
/**
 * Initialize ONNX Embeddings WASM module
 */
export async function initOnnxEmbeddingsWasm() {
    if (initialized)
        return wasmModule !== null;
    if (initPromise)
        return initPromise;
    initPromise = (async () => {
        try {
            const mod = await import('ruvector-onnx-embeddings-wasm');
            wasmModule = mod;
            // Initialize WASM runtime
            if (wasmModule.initialize) {
                await wasmModule.initialize();
            }
            // Load default model
            if (wasmModule.loadModel && !wasmModule.isModelLoaded?.()) {
                await wasmModule.loadModel();
            }
            initialized = true;
            logger.info('ONNX Embeddings WASM initialized', {
                version: wasmModule.getVersion?.() || 'unknown',
                simd: wasmModule.isSIMDEnabled?.() ?? false,
                modelLoaded: wasmModule.isModelLoaded?.() ?? false,
            });
            return true;
        }
        catch (error) {
            logger.debug('ONNX Embeddings WASM not available', { error });
            initialized = true;
            return false;
        }
    })();
    return initPromise;
}
/**
 * Check if ONNX WASM embeddings are available
 */
export function isOnnxWasmAvailable() {
    return wasmModule !== null && (wasmModule.isAvailable?.() ?? false);
}
/**
 * Check if SIMD acceleration is enabled
 */
export function isSIMDEnabled() {
    return wasmModule?.isSIMDEnabled?.() ?? false;
}
/**
 * Generate embedding for text using WASM
 */
export async function embed(text) {
    if (!initialized) {
        await initOnnxEmbeddingsWasm();
    }
    const startTime = performance.now();
    if (wasmModule && wasmModule.isModelLoaded?.()) {
        try {
            const embedding = await wasmModule.embed(text);
            const timeMs = performance.now() - startTime;
            stats.totalEmbeddings++;
            stats.totalLatencyMs += timeMs;
            return {
                embedding,
                timeMs,
                source: 'wasm',
            };
        }
        catch (error) {
            logger.warn('WASM embedding failed, using fallback', { error });
        }
    }
    // Fallback to simple hash-based embedding
    const embedding = simpleEmbed(text);
    const timeMs = performance.now() - startTime;
    stats.totalEmbeddings++;
    stats.totalLatencyMs += timeMs;
    return {
        embedding,
        timeMs,
        source: 'fallback',
    };
}
/**
 * Generate embeddings for multiple texts (batch processing)
 */
export async function embedBatch(texts) {
    if (!initialized) {
        await initOnnxEmbeddingsWasm();
    }
    const startTime = performance.now();
    if (wasmModule && wasmModule.embedBatch && wasmModule.isModelLoaded?.()) {
        try {
            const embeddings = await wasmModule.embedBatch(texts);
            const totalTimeMs = performance.now() - startTime;
            stats.totalEmbeddings += texts.length;
            stats.totalLatencyMs += totalTimeMs;
            return {
                embeddings,
                totalTimeMs,
                avgTimeMs: totalTimeMs / texts.length,
                source: 'wasm',
            };
        }
        catch (error) {
            logger.warn('WASM batch embedding failed, using fallback', { error });
        }
    }
    // Fallback to sequential simple embeddings
    const embeddings = texts.map((text) => simpleEmbed(text));
    const totalTimeMs = performance.now() - startTime;
    stats.totalEmbeddings += texts.length;
    stats.totalLatencyMs += totalTimeMs;
    return {
        embeddings,
        totalTimeMs,
        avgTimeMs: totalTimeMs / texts.length,
        source: 'fallback',
    };
}
/**
 * Compute similarity between two texts
 */
export async function similarity(text1, text2) {
    if (wasmModule && wasmModule.similarity) {
        try {
            return await wasmModule.similarity(text1, text2);
        }
        catch {
            // Fall through to fallback
        }
    }
    // Fallback
    const [result1, result2] = await Promise.all([embed(text1), embed(text2)]);
    return cosineSimilarity(result1.embedding, result2.embedding);
}
/**
 * Compute cosine similarity between two embeddings
 */
export function cosineSimilarity(a, b) {
    if (wasmModule?.cosineSimilarity) {
        return wasmModule.cosineSimilarity(a, b);
    }
    // JS fallback
    let dot = 0;
    let normA = 0;
    let normB = 0;
    for (let i = 0; i < a.length; i++) {
        dot += a[i] * b[i];
        normA += a[i] * a[i];
        normB += b[i] * b[i];
    }
    const denominator = Math.sqrt(normA) * Math.sqrt(normB);
    return denominator === 0 ? 0 : dot / denominator;
}
/**
 * Get embedding statistics
 */
export function getStats() {
    const modelInfo = wasmModule?.getModelInfo?.();
    return {
        available: isOnnxWasmAvailable(),
        simdEnabled: isSIMDEnabled(),
        modelLoaded: wasmModule?.isModelLoaded?.() ?? false,
        modelInfo,
        totalEmbeddings: stats.totalEmbeddings,
        totalLatencyMs: stats.totalLatencyMs,
        avgLatencyMs: stats.totalEmbeddings > 0 ? stats.totalLatencyMs / stats.totalEmbeddings : 0,
    };
}
/**
 * Shutdown WASM module
 */
export async function shutdown() {
    if (wasmModule?.shutdown) {
        await wasmModule.shutdown();
    }
    wasmModule = null;
    initialized = false;
    initPromise = null;
}
/**
 * Reset stats (for testing)
 */
export function resetStats() {
    stats = { totalEmbeddings: 0, totalLatencyMs: 0 };
}
/**
 * Simple hash-based embedding fallback
 * Not semantic, but provides consistent embeddings for testing
 */
function simpleEmbed(text, dim = 384) {
    const embedding = new Float32Array(dim);
    // Multi-pass hash for better distribution
    for (let i = 0; i < text.length; i++) {
        const code = text.charCodeAt(i);
        embedding[i % dim] += code / 255;
        embedding[(i * 7) % dim] += (code * 0.3) / 255;
        embedding[(i * 13) % dim] += (code * 0.2) / 255;
    }
    // Normalize
    let norm = 0;
    for (let i = 0; i < dim; i++) {
        norm += embedding[i] * embedding[i];
    }
    norm = Math.sqrt(norm) || 1;
    for (let i = 0; i < dim; i++) {
        embedding[i] /= norm;
    }
    return embedding;
}
/**
 * OnnxEmbeddingsWasm class for object-oriented usage
 */
export class OnnxEmbeddingsWasm {
    initialized = false;
    async init() {
        this.initialized = await initOnnxEmbeddingsWasm();
        return this.initialized;
    }
    async embed(text) {
        if (!this.initialized)
            await this.init();
        return embed(text);
    }
    async embedBatch(texts) {
        if (!this.initialized)
            await this.init();
        return embedBatch(texts);
    }
    async similarity(text1, text2) {
        if (!this.initialized)
            await this.init();
        return similarity(text1, text2);
    }
    cosineSimilarity(a, b) {
        return cosineSimilarity(a, b);
    }
    isAvailable() {
        return isOnnxWasmAvailable();
    }
    isSIMDEnabled() {
        return isSIMDEnabled();
    }
    getStats() {
        return getStats();
    }
    async shutdown() {
        await shutdown();
        this.initialized = false;
    }
}
// Singleton instance
let instance = null;
/**
 * Get singleton OnnxEmbeddingsWasm instance
 */
export function getOnnxEmbeddingsWasm() {
    if (!instance) {
        instance = new OnnxEmbeddingsWasm();
    }
    return instance;
}
export default {
    OnnxEmbeddingsWasm,
    getOnnxEmbeddingsWasm,
    initOnnxEmbeddingsWasm,
    isOnnxWasmAvailable,
    isSIMDEnabled,
    embed,
    embedBatch,
    similarity,
    cosineSimilarity,
    getStats,
    shutdown,
    resetStats,
};
//# sourceMappingURL=onnx-embeddings-wasm.js.map