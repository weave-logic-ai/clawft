/**
 * EmbeddingService - Text Embedding Generation
 *
 * Handles text-to-vector embedding generation using various models.
 * Supports both local (transformers.js) and remote (OpenAI, etc.) embeddings.
 */
export interface EmbeddingConfig {
    model: string;
    dimension: number;
    provider: 'transformers' | 'openai' | 'local';
    apiKey?: string;
}
export declare class EmbeddingService {
    private config;
    private pipeline;
    private cache;
    constructor(config: EmbeddingConfig);
    /**
     * Initialize the embedding service
     */
    initialize(): Promise<void>;
    /**
     * Generate embedding for text
     */
    embed(text: string): Promise<Float32Array>;
    /**
     * Batch embed multiple texts
     */
    embedBatch(texts: string[]): Promise<Float32Array[]>;
    /**
     * Clear embedding cache
     */
    clearCache(): void;
    private embedOpenAI;
    private mockEmbedding;
}
//# sourceMappingURL=EmbeddingService.d.ts.map