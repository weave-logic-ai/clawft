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
interface ModelInfo {
    name: string;
    dimension: number;
    vocabSize: number;
    maxSequenceLength: number;
}
export interface OnnxEmbeddingResult {
    embedding: Float32Array;
    timeMs: number;
    source: 'wasm' | 'fallback';
}
export interface OnnxBatchResult {
    embeddings: Float32Array[];
    totalTimeMs: number;
    avgTimeMs: number;
    source: 'wasm' | 'fallback';
}
export interface OnnxEmbeddingsStats {
    available: boolean;
    simdEnabled: boolean;
    modelLoaded: boolean;
    modelInfo?: ModelInfo;
    totalEmbeddings: number;
    totalLatencyMs: number;
    avgLatencyMs: number;
}
/**
 * Initialize ONNX Embeddings WASM module
 */
export declare function initOnnxEmbeddingsWasm(): Promise<boolean>;
/**
 * Check if ONNX WASM embeddings are available
 */
export declare function isOnnxWasmAvailable(): boolean;
/**
 * Check if SIMD acceleration is enabled
 */
export declare function isSIMDEnabled(): boolean;
/**
 * Generate embedding for text using WASM
 */
export declare function embed(text: string): Promise<OnnxEmbeddingResult>;
/**
 * Generate embeddings for multiple texts (batch processing)
 */
export declare function embedBatch(texts: string[]): Promise<OnnxBatchResult>;
/**
 * Compute similarity between two texts
 */
export declare function similarity(text1: string, text2: string): Promise<number>;
/**
 * Compute cosine similarity between two embeddings
 */
export declare function cosineSimilarity(a: Float32Array, b: Float32Array): number;
/**
 * Get embedding statistics
 */
export declare function getStats(): OnnxEmbeddingsStats;
/**
 * Shutdown WASM module
 */
export declare function shutdown(): Promise<void>;
/**
 * Reset stats (for testing)
 */
export declare function resetStats(): void;
/**
 * OnnxEmbeddingsWasm class for object-oriented usage
 */
export declare class OnnxEmbeddingsWasm {
    private initialized;
    init(): Promise<boolean>;
    embed(text: string): Promise<OnnxEmbeddingResult>;
    embedBatch(texts: string[]): Promise<OnnxBatchResult>;
    similarity(text1: string, text2: string): Promise<number>;
    cosineSimilarity(a: Float32Array, b: Float32Array): number;
    isAvailable(): boolean;
    isSIMDEnabled(): boolean;
    getStats(): OnnxEmbeddingsStats;
    shutdown(): Promise<void>;
}
/**
 * Get singleton OnnxEmbeddingsWasm instance
 */
export declare function getOnnxEmbeddingsWasm(): OnnxEmbeddingsWasm;
declare const _default: {
    OnnxEmbeddingsWasm: typeof OnnxEmbeddingsWasm;
    getOnnxEmbeddingsWasm: typeof getOnnxEmbeddingsWasm;
    initOnnxEmbeddingsWasm: typeof initOnnxEmbeddingsWasm;
    isOnnxWasmAvailable: typeof isOnnxWasmAvailable;
    isSIMDEnabled: typeof isSIMDEnabled;
    embed: typeof embed;
    embedBatch: typeof embedBatch;
    similarity: typeof similarity;
    cosineSimilarity: typeof cosineSimilarity;
    getStats: typeof getStats;
    shutdown: typeof shutdown;
    resetStats: typeof resetStats;
};
export default _default;
//# sourceMappingURL=onnx-embeddings-wasm.d.ts.map