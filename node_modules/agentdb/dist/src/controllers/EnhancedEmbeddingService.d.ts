/**
 * Enhanced EmbeddingService with WASM Acceleration
 *
 * Extends the base EmbeddingService with WASM-accelerated batch operations
 * and improved performance for large-scale embedding generation.
 */
import { EmbeddingService, EmbeddingConfig } from './EmbeddingService.js';
export interface EnhancedEmbeddingConfig extends EmbeddingConfig {
    enableWASM?: boolean;
    enableBatchProcessing?: boolean;
    batchSize?: number;
}
export declare class EnhancedEmbeddingService extends EmbeddingService {
    private wasmSearch;
    private enhancedConfig;
    constructor(config: EnhancedEmbeddingConfig);
    /**
     * Initialize WASM acceleration
     */
    private initializeWASM;
    /**
     * Enhanced batch embedding with parallel processing
     */
    embedBatch(texts: string[]): Promise<Float32Array[]>;
    /**
     * Calculate similarity between two texts using WASM acceleration
     */
    similarity(textA: string, textB: string): Promise<number>;
    /**
     * Find most similar texts from a corpus
     */
    findMostSimilar(query: string, corpus: string[], k?: number): Promise<Array<{
        text: string;
        similarity: number;
        index: number;
    }>>;
    /**
     * Get service statistics
     */
    getStats(): {
        cacheSize: number;
        wasmEnabled: boolean;
        simdEnabled: boolean;
    };
    /**
     * Cosine similarity fallback
     */
    private cosineSimilarity;
}
//# sourceMappingURL=EnhancedEmbeddingService.d.ts.map