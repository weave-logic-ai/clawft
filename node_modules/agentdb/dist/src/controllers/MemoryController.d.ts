/**
 * MemoryController - Unified Memory Management with Attention Mechanisms
 *
 * Provides a unified interface for memory storage, retrieval, and search
 * with integrated attention mechanisms for enhanced relevance scoring.
 *
 * Features:
 * - CRUD operations for memory entries
 * - Vector similarity search
 * - Attention-enhanced retrieval
 * - Temporal and importance weighting
 * - Multiple namespace support
 * - Integration with vector backends
 */
import type { VectorBackend } from '../backends/VectorBackend.js';
import { SelfAttentionController } from './attention/SelfAttentionController.js';
import { CrossAttentionController } from './attention/CrossAttentionController.js';
import { MultiHeadAttentionController } from './attention/MultiHeadAttentionController.js';
/**
 * Configuration for the MemoryController
 */
export interface MemoryControllerConfig {
    /** Default namespace for memories */
    namespace?: string;
    /** Enable attention-enhanced retrieval */
    enableAttention?: boolean;
    /** Number of attention heads for multi-head attention */
    numHeads?: number;
    /** Default top-k for search operations */
    defaultTopK?: number;
    /** Default similarity threshold */
    defaultThreshold?: number;
}
/**
 * Memory entry structure
 */
export interface Memory {
    /** Unique identifier */
    id: string;
    /** Text content */
    content?: string;
    /** Embedding vector */
    embedding: number[];
    /** Importance score (0-1) */
    importance?: number;
    /** Creation timestamp */
    timestamp?: number;
    /** Additional metadata */
    metadata?: Record<string, any>;
}
/**
 * Search options for memory retrieval
 */
export interface SearchOptions {
    /** Number of top results to return */
    topK?: number;
    /** Minimum similarity threshold */
    threshold?: number;
    /** Metadata filters */
    filter?: Record<string, any>;
    /** Use attention for ranking */
    useAttention?: boolean;
    /** Weight recent memories higher */
    temporalWeight?: number;
    /** Weight by importance score */
    weighByImportance?: boolean;
}
/**
 * Search result with scores
 */
export interface SearchResult extends Memory {
    /** Similarity score */
    score: number;
    /** Attention score (if attention enabled) */
    attentionScore?: number;
}
/**
 * Result from attention-enhanced retrieval
 */
export interface AttentionRetrievalResult extends SearchResult {
    /** Combined attention score from all mechanisms */
    attentionScore: number;
}
/**
 * MemoryController - Main class for memory management
 */
export declare class MemoryController {
    private vectorBackend;
    private memories;
    private config;
    private selfAttention;
    private crossAttention;
    private multiHeadAttention;
    constructor(vectorBackend?: VectorBackend | null, config?: MemoryControllerConfig);
    /**
     * Store a memory entry
     */
    store(memory: Memory, namespace?: string): Promise<void>;
    /**
     * Retrieve a memory by ID
     */
    retrieve(id: string): Promise<Memory | undefined>;
    /**
     * Update an existing memory
     */
    update(id: string, updates: Partial<Memory>): Promise<boolean>;
    /**
     * Delete a memory by ID
     */
    delete(id: string): Promise<boolean>;
    /**
     * Search for similar memories
     */
    search(query: number[], options?: SearchOptions): Promise<SearchResult[]>;
    /**
     * Retrieve memories with attention-enhanced scoring
     */
    retrieveWithAttention(query: number[], options?: SearchOptions): Promise<AttentionRetrievalResult[]>;
    /**
     * Compute cosine similarity between two vectors
     */
    private cosineSimilarity;
    /**
     * Check if metadata matches filter criteria
     */
    private matchesFilter;
    /**
     * Get all memories (for iteration/export)
     */
    getAllMemories(): Memory[];
    /**
     * Get memory count
     */
    get count(): number;
    /**
     * Clear all memories
     */
    clear(): void;
    /**
     * Get the self-attention controller for direct access
     */
    getSelfAttentionController(): SelfAttentionController;
    /**
     * Get the cross-attention controller for direct access
     */
    getCrossAttentionController(): CrossAttentionController;
    /**
     * Get the multi-head attention controller for direct access
     */
    getMultiHeadAttentionController(): MultiHeadAttentionController;
    /**
     * Get controller statistics
     */
    getStats(): {
        memoryCount: number;
        selfAttention: ReturnType<SelfAttentionController['getStats']>;
        crossAttention: ReturnType<CrossAttentionController['getStats']>;
        multiHeadAttention: ReturnType<MultiHeadAttentionController['getStats']>;
    };
}
//# sourceMappingURL=MemoryController.d.ts.map