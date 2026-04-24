/**
 * Enhanced Embedding Service with RuVector Integration
 *
 * A comprehensive embedding service supporting:
 * - Multiple providers (@xenova/transformers, OpenAI, Cohere, custom)
 * - LRU cache with O(1) operations using doubly-linked list
 * - Batch processing with semaphore-controlled parallelism
 * - Text pre-processing pipeline (normalization, chunking, deduplication)
 * - Direct RuVector storage and hybrid search
 * - Lazy model loading with runtime switching
 */
import type { VectorBackend } from '../backends/VectorBackend.js';
/** @inline Maximum allowed vector dimension for bounds checking */
export declare const MAX_VECTOR_DIMENSION = 4096;
/** @inline Maximum allowed cache size to prevent memory exhaustion */
export declare const MAX_CACHE_SIZE = 100000;
/** @inline Default cache size for embedding storage */
export declare const DEFAULT_CACHE_SIZE = 10000;
/** @inline Maximum batch size to prevent memory exhaustion */
export declare const MAX_BATCH_SIZE = 10000;
/**
 * Supported embedding models
 */
export type SupportedModel = 'all-MiniLM-L6-v2' | 'all-mpnet-base-v2' | 'bge-small-en-v1.5' | string;
/**
 * Embedding provider types
 */
export type EmbeddingProvider = 'transformers' | 'openai' | 'cohere' | 'custom';
/**
 * Configuration for the enhanced embedding service (user input)
 */
export interface EnhancedEmbeddingConfig {
    /** Embedding provider (default: 'transformers') */
    provider?: EmbeddingProvider;
    /** Model name (default: 'all-MiniLM-L6-v2') */
    model?: SupportedModel;
    /** Vector dimension (auto-detected from model if not specified) */
    dimension?: number;
    /** API key for OpenAI/Cohere providers */
    apiKey?: string;
    /** Custom embedding function for 'custom' provider */
    customEmbedder?: CustomEmbedder;
    /** LRU cache configuration */
    cache?: Partial<CacheConfig>;
    /** Batch processing configuration */
    batch?: Partial<BatchConfig>;
    /** Pre-processing pipeline configuration */
    preprocessing?: Partial<PreprocessingConfig>;
    /** RuVector backend for direct storage */
    vectorBackend?: VectorBackend;
}
/**
 * Cache configuration
 */
export interface CacheConfig {
    /** Maximum number of cached embeddings (default: 10000) */
    maxSize: number;
    /** Enable cache statistics tracking (default: true) */
    trackStats: boolean;
}
/**
 * Batch processing configuration
 */
export interface BatchConfig {
    /** Batch size for processing (default: 32) */
    batchSize: number;
    /** Maximum concurrent batches (default: 4) */
    maxConcurrency: number;
    /** Enable progress callbacks (default: true) */
    enableProgress: boolean;
}
/**
 * Pre-processing pipeline configuration
 */
export interface PreprocessingConfig {
    /** Enable text normalization (default: true) */
    normalize: boolean;
    /** Maximum text length before chunking (default: 512 tokens approx) */
    maxLength: number;
    /** Chunk overlap for long texts (default: 50 chars) */
    chunkOverlap: number;
    /** Enable deduplication (default: true) */
    deduplicate: boolean;
}
/**
 * Custom embedding function signature
 */
export type CustomEmbedder = (text: string) => Promise<Float32Array>;
/**
 * Search result from the embedding service
 */
export interface SearchResult {
    /** Unique identifier */
    id: string;
    /** Original text (if stored) */
    text?: string;
    /** Similarity score (0-1, higher is better) */
    similarity: number;
    /** Raw distance from vector search */
    distance: number;
    /** Associated metadata */
    metadata?: Record<string, unknown>;
}
/**
 * Statistics for the embedding service
 */
export interface EmbeddingStats {
    /** Total embeddings generated */
    totalEmbeddings: number;
    /** Cache statistics */
    cache: {
        hits: number;
        misses: number;
        hitRate: number;
        size: number;
        maxSize: number;
    };
    /** Batch processing statistics */
    batch: {
        totalBatches: number;
        averageBatchSize: number;
    };
    /** Model information */
    model: {
        name: string;
        dimension: number;
        provider: EmbeddingProvider;
        loaded: boolean;
    };
    /** Vector backend statistics (if connected) */
    vectorBackend?: {
        count: number;
        backend: string;
    };
}
/**
 * Progress callback for batch operations
 */
export type ProgressCallback = (progress: {
    current: number;
    total: number;
    percentage: number;
}) => void;
/**
 * Enhanced Embedding Service with RuVector Integration
 */
export declare class EnhancedEmbeddingService {
    private config;
    private cache;
    private semaphore;
    private pipeline;
    private modelLoaded;
    private totalEmbeddings;
    private totalBatches;
    private totalBatchItems;
    constructor(config?: EnhancedEmbeddingConfig);
    /**
     * Generate embedding for a single text
     */
    embed(text: string): Promise<Float32Array>;
    /**
     * Generate embeddings for multiple texts with batch processing
     */
    embedBatch(texts: string[], onProgress?: ProgressCallback): Promise<Float32Array[]>;
    /**
     * Search for similar texts using RuVector backend
     */
    search(query: string, k?: number): Promise<SearchResult[]>;
    /**
     * Store text with embedding in RuVector
     */
    store(id: string, text: string, metadata?: Record<string, unknown>): Promise<void>;
    /**
     * Store multiple texts with embeddings in RuVector (batch operation)
     */
    storeBatch(items: Array<{
        id: string;
        text: string;
        metadata?: Record<string, unknown>;
    }>, onProgress?: ProgressCallback): Promise<void>;
    /**
     * Hybrid search combining embedding similarity with metadata filters
     */
    hybridSearch(query: string, k?: number, filter?: Record<string, unknown>): Promise<SearchResult[]>;
    /**
     * Lazy load the embedding model
     */
    loadModel(): Promise<void>;
    /**
     * Switch to a different model at runtime
     */
    switchModel(model: SupportedModel, provider?: EmbeddingProvider): Promise<void>;
    /**
     * Get service statistics
     */
    getStats(): EmbeddingStats;
    /**
     * Clear the embedding cache
     */
    clearCache(): void;
    /**
     * Get current model dimension
     */
    getDimension(): number;
    /**
     * Pre-process text according to config
     */
    private preprocessText;
    /**
     * Chunk long text into smaller pieces with overlap
     */
    chunkText(text: string): string[];
    /**
     * Generate cache key for text
     */
    private getCacheKey;
    /**
     * Deduplicate texts and create mapping
     */
    private deduplicateTexts;
    /**
     * Generate embedding using configured provider
     */
    private generateEmbedding;
    /**
     * Process a batch of texts
     */
    private processBatch;
    /**
     * Load transformers.js model
     */
    private loadTransformersModel;
    /**
     * Embed with @xenova/transformers (single)
     */
    private embedWithTransformers;
    /**
     * Batch embed with transformers
     */
    private batchEmbedWithTransformers;
    /**
     * Embed with OpenAI API
     */
    private embedWithOpenAI;
    /**
     * Batch embed with OpenAI API
     */
    private batchEmbedWithOpenAI;
    /**
     * Embed with Cohere API
     */
    private embedWithCohere;
    /**
     * Batch embed with Cohere API
     */
    private batchEmbedWithCohere;
    /**
     * Embed with custom function
     */
    private embedWithCustom;
    /**
     * Generate mock embedding for testing/fallback
     */
    private mockEmbedding;
}
/**
 * Create an enhanced embedding service with default configuration
 */
export declare function createEmbeddingService(config?: EnhancedEmbeddingConfig): EnhancedEmbeddingService;
export default EnhancedEmbeddingService;
//# sourceMappingURL=enhanced-embeddings.d.ts.map