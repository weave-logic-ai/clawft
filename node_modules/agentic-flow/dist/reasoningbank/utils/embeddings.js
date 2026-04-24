/**
 * Embedding generation for semantic similarity
 * Uses local transformers.js - no API key required!
 */
import { pipeline, env } from '@xenova/transformers';
import { loadConfig } from './config.js';
// Configure transformers.js to use WASM backend only (avoid ONNX runtime issues)
// The native ONNX runtime causes "DefaultLogger not registered" errors in Node.js
env.backends.onnx.wasm.proxy = false; // Disable ONNX runtime proxy
env.backends.onnx.wasm.numThreads = 1; // Single thread for stability
let embeddingPipeline = null;
let isInitializing = false;
const embeddingCache = new Map();
/**
 * Initialize the embedding pipeline (lazy load)
 */
async function initializeEmbeddings() {
    if (embeddingPipeline)
        return;
    if (isInitializing) {
        // Wait for initialization to complete
        while (isInitializing) {
            await new Promise(resolve => setTimeout(resolve, 100));
        }
        return;
    }
    // Detect npx environment (known transformer initialization issues)
    const isNpxEnv = process.env.npm_lifecycle_event === 'npx' ||
        process.env.npm_execpath?.includes('npx') ||
        process.cwd().includes('/_npx/') ||
        process.cwd().includes('\\_npx\\');
    if (isNpxEnv && !process.env.FORCE_TRANSFORMERS) {
        console.log('[Embeddings] NPX environment detected - using hash-based embeddings');
        console.log('[Embeddings] For semantic search, install globally: npm install -g claude-flow');
        isInitializing = false;
        return;
    }
    isInitializing = true;
    console.log('[Embeddings] Initializing local embedding model (Xenova/all-MiniLM-L6-v2)...');
    console.log('[Embeddings] First run will download ~23MB model...');
    try {
        embeddingPipeline = await pipeline('feature-extraction', 'Xenova/all-MiniLM-L6-v2', { quantized: true } // Smaller, faster
        );
        console.log('[Embeddings] Local model ready! (384 dimensions)');
    }
    catch (error) {
        console.error('[Embeddings] Failed to initialize:', error?.message || error);
        console.warn('[Embeddings] Falling back to hash-based embeddings');
    }
    finally {
        isInitializing = false;
    }
}
/**
 * Compute embedding for text using local model
 */
export async function computeEmbedding(text) {
    const config = loadConfig();
    // Check cache
    const cacheKey = `local:${text}`;
    if (embeddingCache.has(cacheKey)) {
        return embeddingCache.get(cacheKey);
    }
    let embedding;
    // Initialize if needed
    await initializeEmbeddings();
    if (embeddingPipeline) {
        try {
            // Use transformers.js for real embeddings
            const output = await embeddingPipeline(text, {
                pooling: 'mean',
                normalize: true
            });
            embedding = new Float32Array(output.data);
        }
        catch (error) {
            console.error('[Embeddings] Generation failed:', error?.message || error);
            embedding = hashEmbed(text, 384); // Fallback
        }
    }
    else {
        // Fallback to hash-based embeddings
        const dims = config?.embeddings?.dimensions || 384;
        embedding = hashEmbed(text, dims);
    }
    // Cache with LRU (limit 1000 entries)
    if (embeddingCache.size > 1000) {
        const firstKey = embeddingCache.keys().next().value;
        if (firstKey) {
            embeddingCache.delete(firstKey);
        }
    }
    embeddingCache.set(cacheKey, embedding);
    // Set TTL for cache entry
    const ttl = config?.embeddings?.cache_ttl_seconds || 3600;
    setTimeout(() => embeddingCache.delete(cacheKey), ttl * 1000);
    return embedding;
}
/**
 * Batch compute embeddings (more efficient)
 */
export async function computeEmbeddingBatch(texts) {
    return Promise.all(texts.map(text => computeEmbedding(text)));
}
/**
 * Get embedding dimensions
 */
export function getEmbeddingDimensions() {
    return 384; // all-MiniLM-L6-v2 uses 384 dimensions
}
/**
 * Deterministic hash-based embedding (fallback)
 */
function hashEmbed(text, dims) {
    const hash = simpleHash(text);
    const vec = new Float32Array(dims);
    // Generate deterministic pseudo-random vector from hash
    for (let i = 0; i < dims; i++) {
        vec[i] = Math.sin(hash * (i + 1) * 0.01) + Math.cos(hash * i * 0.02);
    }
    return normalize(vec);
}
/**
 * Simple string hash function
 */
function simpleHash(str) {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
        hash = ((hash << 5) - hash) + str.charCodeAt(i);
        hash |= 0;
    }
    return Math.abs(hash);
}
/**
 * Normalize vector to unit length
 */
function normalize(vec) {
    let mag = 0;
    for (let i = 0; i < vec.length; i++) {
        mag += vec[i] * vec[i];
    }
    mag = Math.sqrt(mag);
    if (mag === 0)
        return vec;
    for (let i = 0; i < vec.length; i++) {
        vec[i] /= mag;
    }
    return vec;
}
/**
 * Clear embedding cache
 */
export function clearEmbeddingCache() {
    embeddingCache.clear();
}
//# sourceMappingURL=embeddings.js.map