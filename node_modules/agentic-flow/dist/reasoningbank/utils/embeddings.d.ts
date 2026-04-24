/**
 * Embedding generation for semantic similarity
 * Uses local transformers.js - no API key required!
 */
/**
 * Compute embedding for text using local model
 */
export declare function computeEmbedding(text: string): Promise<Float32Array>;
/**
 * Batch compute embeddings (more efficient)
 */
export declare function computeEmbeddingBatch(texts: string[]): Promise<Float32Array[]>;
/**
 * Get embedding dimensions
 */
export declare function getEmbeddingDimensions(): number;
/**
 * Clear embedding cache
 */
export declare function clearEmbeddingCache(): void;
//# sourceMappingURL=embeddings.d.ts.map