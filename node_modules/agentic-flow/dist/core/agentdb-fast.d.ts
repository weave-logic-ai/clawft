/**
 * AgentDB Fast API
 *
 * Provides programmatic access to AgentDB without CLI overhead
 * Eliminates 2.3s overhead from process spawning and transformers.js init
 */
import { EventEmitter } from 'events';
export interface Episode {
    id?: string;
    sessionId: string;
    task: string;
    trajectory: string[];
    reward: number;
    quality?: number;
    embedding?: number[];
    context?: Record<string, any>;
    timestamp?: number;
}
export interface EpisodeSearchOptions {
    task?: string;
    minReward?: number;
    maxReward?: number;
    sessionId?: string;
    k?: number;
    filter?: Record<string, any>;
}
export interface Pattern {
    id?: string;
    task: string;
    input: string;
    output: string;
    quality: number;
    embedding?: number[];
    context?: Record<string, any>;
    timestamp?: number;
}
/**
 * Fast AgentDB client that avoids CLI overhead
 *
 * Performance:
 * - CLI: ~2,350ms per operation
 * - Direct API: ~10-50ms per operation
 * - Speedup: ~50-200x faster
 */
export declare class AgentDBFast extends EventEmitter {
    private db;
    private backend;
    private config;
    private initialized;
    private embeddingCache;
    constructor(config?: {
        path?: string;
        vectorDimensions?: number;
        enableHNSW?: boolean;
        hnswM?: number;
        hnswEfConstruction?: number;
    });
    /**
     * Initialize database connection (lazy)
     */
    initialize(): Promise<void>;
    /**
     * Store an episode (fast, no CLI overhead)
     */
    storeEpisode(episode: Episode): Promise<string>;
    /**
     * Retrieve episodes by task similarity (fast)
     */
    retrieveEpisodes(options: EpisodeSearchOptions): Promise<Episode[]>;
    /**
     * Store a pattern (for ReasoningBank)
     */
    storePattern(pattern: Pattern): Promise<string>;
    /**
     * Search for similar patterns
     */
    searchPatterns(query: string, k?: number, minQuality?: number): Promise<Pattern[]>;
    /**
     * Get database statistics
     */
    getStats(): Promise<{
        totalVectors: number;
        totalEpisodes: number;
        totalPatterns: number;
        avgQuality: number;
    }>;
    /**
     * Close database connection
     */
    close(): Promise<void>;
    /**
     * Generate embedding for text (with caching)
     *
     * Note: This is a simple mock. In production, replace with:
     * - OpenAI embeddings API
     * - Local transformer model
     * - SentenceTransformers
     */
    private getEmbedding;
    /**
     * Simple hash-based embedding (MOCK - REPLACE IN PRODUCTION)
     *
     * Production alternatives:
     * 1. OpenAI: https://platform.openai.com/docs/guides/embeddings
     * 2. Transformers.js: https://huggingface.co/docs/transformers.js
     * 3. SBERT: https://www.sbert.net/
     */
    private simpleHashEmbedding;
    /**
     * Generate unique ID
     */
    private generateId;
}
/**
 * Convenience function to create a fast AgentDB client
 */
export declare function createFastAgentDB(config?: {
    path?: string;
    vectorDimensions?: number;
    enableHNSW?: boolean;
}): AgentDBFast;
/**
 * Performance comparison helper
 */
export declare function benchmarkAgentDB(): Promise<{
    cli: {
        store: number;
        retrieve: number;
    };
    api: {
        store: number;
        retrieve: number;
    };
    speedup: {
        store: number;
        retrieve: number;
    };
}>;
//# sourceMappingURL=agentdb-fast.d.ts.map