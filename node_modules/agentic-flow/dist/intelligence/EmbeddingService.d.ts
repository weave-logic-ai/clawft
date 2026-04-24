/**
 * EmbeddingService - Unified embedding interface for agentic-flow
 *
 * Uses ruvector@0.1.61+ for ONNX embeddings with:
 * - SIMD128 acceleration (6x faster)
 * - Parallel worker threads (7 workers)
 * - all-MiniLM-L6-v2 model (384 dimensions)
 * - Persistent SQLite cache (0.1ms vs 400ms)
 *
 * Configure via:
 * - AGENTIC_FLOW_EMBEDDINGS=simple|onnx|auto (default: auto)
 * - AGENTIC_FLOW_EMBEDDING_MODEL=all-MiniLM-L6-v2 (default)
 * - AGENTIC_FLOW_EMBEDDING_CACHE=true|false (default: true)
 * - AGENTIC_FLOW_PERSISTENT_CACHE=true|false (default: true)
 */
export type EmbeddingBackend = 'simple' | 'onnx' | 'auto';
export interface EmbeddingStats {
    backend: EmbeddingBackend;
    effectiveBackend: EmbeddingBackend;
    dimension: number;
    totalEmbeddings: number;
    totalLatencyMs: number;
    avgLatencyMs: number;
    cacheHits: number;
    modelLoaded: boolean;
    modelName?: string;
    simdAvailable?: boolean;
    parallelWorkers?: number;
    persistentCache?: {
        enabled: boolean;
        entries: number;
        hits: number;
        misses: number;
        hitRate: number;
        dbSizeKB: number;
    };
}
export interface SimilarityResult {
    similarity: number;
    timeMs: number;
}
export interface SearchResult {
    text: string;
    index: number;
    similarity: number;
}
export interface DuplicateGroup {
    indices: number[];
    texts: string[];
    similarity: number;
}
export declare class EmbeddingService {
    private static instance;
    private backend;
    private effectiveBackend;
    private dimension;
    private modelName;
    private modelLoaded;
    private loadingPromise;
    private totalEmbeddings;
    private totalLatencyMs;
    private cacheHits;
    private cache;
    private cacheEnabled;
    private persistentCache;
    private persistentCacheEnabled;
    private corpus;
    private constructor();
    static getInstance(): EmbeddingService;
    /**
     * Resolve the effective backend based on ONNX detection
     */
    private resolveBackend;
    /**
     * Get configured backend (may be 'auto')
     */
    getBackend(): EmbeddingBackend;
    /**
     * Get effective backend after detection
     */
    getEffectiveBackend(): EmbeddingBackend;
    /**
     * Get embedding dimension
     */
    getDimension(): number;
    /**
     * Check if ONNX model is loaded
     */
    isModelLoaded(): boolean;
    /**
     * Generate embedding for text
     * Auto-detects ONNX and uses it if available (default behavior)
     */
    embed(text: string): Promise<Float32Array>;
    /**
     * Generate embeddings for multiple texts (batch processing with parallel workers)
     * Batch processing provides significant speedup with parallel ONNX workers
     */
    embedBatch(texts: string[]): Promise<Float32Array[]>;
    /**
     * Compute similarity between two texts
     */
    similarity(text1: string, text2: string): Promise<number>;
    /**
     * Compute NxN similarity matrix for a list of texts
     * Uses parallel workers for ONNX backend
     */
    similarityMatrix(texts: string[]): Promise<number[][]>;
    /**
     * Build a corpus for semantic search
     */
    buildCorpus(texts: string[]): Promise<void>;
    /**
     * Semantic search against the corpus
     * Returns top-k most similar texts
     */
    semanticSearch(query: string, topK?: number): Promise<SearchResult[]>;
    /**
     * Find near-duplicate texts in a list
     * Groups texts with similarity above threshold
     */
    findDuplicates(texts: string[], threshold?: number): Promise<DuplicateGroup[]>;
    /**
     * K-means clustering of texts
     * Returns cluster assignments and centroids
     */
    clusterTexts(texts: string[], k?: number, maxIterations?: number): Promise<{
        clusters: number[];
        centroids: Float32Array[];
    }>;
    /**
     * Stream embeddings for large batches (memory efficient)
     * Yields embeddings one at a time
     */
    streamEmbed(texts: string[], batchSize?: number): AsyncGenerator<{
        index: number;
        text: string;
        embedding: Float32Array;
    }>;
    /**
     * Simple hash-based embedding (fast, not semantic)
     */
    simpleEmbed(text: string, dim?: number): Float32Array;
    /**
     * Compute cosine similarity between two embeddings
     */
    cosineSimilarity(a: Float32Array, b: Float32Array): number;
    /**
     * Get statistics
     */
    getStats(): EmbeddingStats;
    /**
     * Clear in-memory cache
     */
    clearCache(): void;
    /**
     * Clear persistent cache (SQLite)
     */
    clearPersistentCache(): void;
    /**
     * Clear all caches (memory + persistent)
     */
    clearAllCaches(): void;
    /**
     * Get persistent cache stats
     */
    getPersistentCacheStats(): {
        entries: number;
        hits: number;
        misses: number;
        hitRate: number;
    } | null;
    /**
     * Clear corpus
     */
    clearCorpus(): void;
    /**
     * Shutdown (cleanup workers)
     */
    shutdown(): Promise<void>;
    /**
     * Reset instance (for testing)
     */
    static reset(): Promise<void>;
    /**
     * Pretrain cache with texts from files
     * Embeds content and stores in persistent cache for fast retrieval
     *
     * @param sources - File paths or glob patterns, or array of texts
     * @param options - Pretrain options
     * @returns Stats about pretraining
     */
    pretrain(sources: string | string[], options?: {
        batchSize?: number;
        onProgress?: (processed: number, total: number) => void;
        chunkSize?: number;
        overlapSize?: number;
        skipCached?: boolean;
    }): Promise<{
        processed: number;
        cached: number;
        skipped: number;
        timeMs: number;
    }>;
    /**
     * Pretrain with common programming patterns
     * Pre-caches embeddings for frequently used code patterns
     */
    pretrainCodePatterns(): Promise<{
        cached: number;
        timeMs: number;
    }>;
    /**
     * Pretrain from repository structure
     * Analyzes file names and paths to pre-cache common patterns
     */
    pretrainFromRepo(repoPath?: string): Promise<{
        files: number;
        chunks: number;
        timeMs: number;
    }>;
    /**
     * Incremental pretrain - only process changed files since last run
     * Uses git diff to detect modified files
     */
    pretrainIncremental(options?: {
        since?: string;
        repoPath?: string;
    }): Promise<{
        changedFiles: number;
        newChunks: number;
        skipped: number;
        timeMs: number;
    }>;
    /**
     * Smart chunking - split code by semantic boundaries
     * (functions, classes, etc.) instead of fixed size
     */
    semanticChunk(content: string, fileType: string): string[];
    /**
     * Pretrain with semantic chunking
     * Uses code structure to create meaningful chunks
     */
    pretrainSemantic(sources: string[], options?: {
        batchSize?: number;
        onProgress?: (processed: number, total: number) => void;
    }): Promise<{
        files: number;
        chunks: number;
        timeMs: number;
    }>;
    /**
     * Priority pretrain - cache most frequently used patterns first
     * Tracks access patterns and prioritizes high-frequency queries
     */
    private accessCounts;
    recordAccess(text: string): void;
    getTopPatterns(n?: number): string[];
    pretrainPriority(n?: number): Promise<{
        cached: number;
        timeMs: number;
    }>;
    /**
     * Warmup cache on session start
     * Combines code patterns + recent repo changes
     */
    warmup(repoPath?: string): Promise<{
        patterns: number;
        recentChanges: number;
        timeMs: number;
    }>;
    /**
     * Intelligent pretrain using ruvector worker pool
     * Analyzes repo structure, code patterns, and prepares cache
     * Uses parallel workers for maximum throughput
     */
    pretrainIntelligent(options?: {
        repoPath?: string;
        parallel?: boolean;
        onProgress?: (stage: string, progress: number) => void;
    }): Promise<{
        stages: {
            codePatterns: {
                count: number;
                timeMs: number;
            };
            astAnalysis: {
                files: number;
                functions: number;
                timeMs: number;
            };
            gitHistory: {
                commits: number;
                hotFiles: number;
                timeMs: number;
            };
            dependencies: {
                modules: number;
                imports: number;
                timeMs: number;
            };
            semanticChunks: {
                chunks: number;
                timeMs: number;
            };
        };
        totalCached: number;
        totalTimeMs: number;
    }>;
    /**
     * Background pretrain - runs in worker if available
     * Non-blocking, returns immediately with a promise
     */
    pretrainBackground(options?: {
        repoPath?: string;
    }): {
        promise: Promise<void>;
        cancel: () => void;
    };
    /**
     * AI-enhanced pretrain using ruvector attention mechanisms
     * Uses HyperbolicAttention for code structure, MoE for routing
     */
    pretrainWithAI(options?: {
        repoPath?: string;
        attentionType?: 'hyperbolic' | 'moe' | 'graph' | 'auto';
        onProgress?: (stage: string, detail: string) => void;
    }): Promise<{
        patterns: {
            type: string;
            count: number;
        }[];
        attention: {
            type: string;
            timeMs: number;
        };
        predictions: {
            prefetch: number;
            confidence: number;
        };
        totalCached: number;
        totalTimeMs: number;
    }>;
    /**
     * Context-aware prefetch using attention
     * Predicts what embeddings will be needed based on current context
     */
    prefetchForContext(context: {
        currentFile?: string;
        recentFiles?: string[];
        taskType?: 'edit' | 'review' | 'debug' | 'test' | 'refactor';
        userQuery?: string;
    }): Promise<{
        prefetched: number;
        confidence: number;
        timeMs: number;
    }>;
}
export declare function getEmbeddingService(): EmbeddingService;
export declare function embed(text: string): Promise<Float32Array>;
export declare function embedBatch(texts: string[]): Promise<Float32Array[]>;
export declare function pretrainCodePatterns(): Promise<{
    cached: number;
    timeMs: number;
}>;
export declare function pretrainFromRepo(repoPath?: string): Promise<{
    files: number;
    chunks: number;
    timeMs: number;
}>;
export declare function textSimilarity(text1: string, text2: string): Promise<number>;
export declare function simpleEmbed(text: string, dim?: number): Float32Array;
export declare function similarityMatrix(texts: string[]): Promise<number[][]>;
export declare function semanticSearch(query: string, topK?: number): Promise<SearchResult[]>;
export declare function findDuplicates(texts: string[], threshold?: number): Promise<DuplicateGroup[]>;
export declare function clusterTexts(texts: string[], k?: number): Promise<{
    clusters: number[];
    centroids: Float32Array[];
}>;
//# sourceMappingURL=EmbeddingService.d.ts.map