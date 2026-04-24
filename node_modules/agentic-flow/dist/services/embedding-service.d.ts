/**
 * Production Embedding Service
 *
 * Replaces mock embeddings with real implementations:
 * 1. OpenAI Embeddings API (text-embedding-3-small/large)
 * 2. Local Transformers.js (runs in Node.js/browser)
 * 3. Custom ONNX models
 * 4. Fallback hash-based embeddings (for development)
 */
import { EventEmitter } from 'events';
export interface EmbeddingConfig {
    provider: 'openai' | 'transformers' | 'onnx' | 'mock';
    model?: string;
    dimensions?: number;
    apiKey?: string;
    cacheSize?: number;
}
export interface EmbeddingResult {
    embedding: number[];
    usage?: {
        promptTokens: number;
        totalTokens: number;
    };
    latency: number;
}
/**
 * Base embedding service interface
 */
export declare abstract class EmbeddingService extends EventEmitter {
    protected config: EmbeddingConfig;
    protected cache: Map<string, number[]>;
    constructor(config: EmbeddingConfig);
    abstract embed(text: string): Promise<EmbeddingResult>;
    abstract embedBatch(texts: string[]): Promise<EmbeddingResult[]>;
    /**
     * Get cached embedding if available
     */
    protected getCached(text: string): number[] | null;
    /**
     * Cache embedding with LRU eviction
     */
    protected setCached(text: string, embedding: number[]): void;
    /**
     * Clear cache
     */
    clearCache(): void;
}
/**
 * OpenAI Embeddings Service
 *
 * Uses OpenAI's text-embedding-3-small (1536D) or text-embedding-3-large (3072D)
 * https://platform.openai.com/docs/guides/embeddings
 */
export declare class OpenAIEmbeddingService extends EmbeddingService {
    private apiKey;
    private model;
    private baseURL;
    constructor(config: Omit<EmbeddingConfig, 'provider'> & {
        apiKey: string;
    });
    embed(text: string): Promise<EmbeddingResult>;
    embedBatch(texts: string[]): Promise<EmbeddingResult[]>;
}
/**
 * Transformers.js Local Embedding Service
 *
 * Runs locally without API calls using ONNX runtime
 * https://huggingface.co/docs/transformers.js
 */
export declare class TransformersEmbeddingService extends EmbeddingService {
    private pipeline;
    private modelName;
    constructor(config: Omit<EmbeddingConfig, 'provider'>);
    initialize(): Promise<void>;
    embed(text: string): Promise<EmbeddingResult>;
    embedBatch(texts: string[]): Promise<EmbeddingResult[]>;
}
/**
 * Mock Embedding Service (for development/testing)
 *
 * Generates deterministic hash-based embeddings
 * Fast but not semantically meaningful
 */
export declare class MockEmbeddingService extends EmbeddingService {
    constructor(config?: Partial<EmbeddingConfig>);
    embed(text: string): Promise<EmbeddingResult>;
    embedBatch(texts: string[]): Promise<EmbeddingResult[]>;
    private hashEmbedding;
}
/**
 * Factory function to create appropriate embedding service
 */
export declare function createEmbeddingService(config: EmbeddingConfig): EmbeddingService;
/**
 * Convenience function for quick embeddings
 */
export declare function getEmbedding(text: string, config?: Partial<EmbeddingConfig>): Promise<number[]>;
/**
 * Benchmark different embedding providers
 */
export declare function benchmarkEmbeddings(testText?: string): Promise<{
    mock: {
        latency: number;
        dimensions: number;
    };
    transformers?: {
        latency: number;
        dimensions: number;
        error?: string;
    };
    openai?: {
        latency: number;
        dimensions: number;
        error?: string;
    };
}>;
//# sourceMappingURL=embedding-service.d.ts.map