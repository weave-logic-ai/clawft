/**
 * ReasoningBank Controller - Pattern Storage and Retrieval
 *
 * Manages reasoning patterns with embeddings for semantic similarity search.
 * Integrates with ReasoningBank WASM for high-performance pattern matching.
 *
 * Pattern Structure:
 * - taskType: Type of task (e.g., "code_review", "data_analysis")
 * - approach: Description of the reasoning approach used
 * - successRate: Success rate of this pattern (0-1)
 * - embedding: Vector embedding of the pattern for similarity search
 * - metadata: Additional contextual information
 *
 * AgentDB v2 Migration:
 * - Uses VectorBackend abstraction for 8x faster search (RuVector/hnswlib)
 * - Optional GNN enhancement via LearningBackend
 * - 100% backward compatible with v1 API
 * - New features: useGNN option, recordOutcome for learning
 */
import type { IDatabaseConnection } from '../types/database.types.js';
import { EmbeddingService } from './EmbeddingService.js';
import type { VectorBackend } from '../backends/VectorBackend.js';
export interface ReasoningPattern {
    id?: number;
    taskType: string;
    approach: string;
    successRate: number;
    embedding?: Float32Array;
    uses?: number;
    avgReward?: number;
    tags?: string[];
    metadata?: Record<string, any>;
    createdAt?: number;
    similarity?: number;
}
export interface PatternSearchQuery {
    /** v1 API: Task string (will be embedded automatically) */
    task?: string;
    /** v2 API: Pre-computed embedding */
    taskEmbedding?: Float32Array;
    k?: number;
    threshold?: number;
    /** Enable GNN-based query enhancement (requires LearningBackend) */
    useGNN?: boolean;
    filters?: {
        taskType?: string;
        minSuccessRate?: number;
        tags?: string[];
    };
}
export interface PatternStats {
    totalPatterns: number;
    avgSuccessRate: number;
    avgUses: number;
    topTaskTypes: Array<{
        taskType: string;
        count: number;
    }>;
    recentPatterns: number;
    highPerformingPatterns: number;
}
/**
 * Optional GNN Learning Backend for query enhancement
 */
export interface LearningBackend {
    /**
     * Enhance query embedding using GNN and neighbor context
     */
    enhance(query: Float32Array, neighbors: Float32Array[], weights: number[]): Float32Array;
    /**
     * Add training sample for future learning
     */
    addSample(embedding: Float32Array, success: boolean): void;
    /**
     * Train the GNN model
     */
    train(options?: {
        epochs?: number;
        batchSize?: number;
    }): Promise<{
        epochs: number;
        finalLoss: number;
    }>;
}
export declare class ReasoningBank {
    private db;
    private embedder;
    private cache;
    private vectorBackend?;
    private learningBackend?;
    private idMapping;
    private nextVectorId;
    /**
     * Constructor supports both legacy (v1) and new (v2) modes
     *
     * Legacy mode (v1 - backward compatible):
     *   new ReasoningBank(db, embedder)
     *
     * New mode (v2 - with VectorBackend):
     *   new ReasoningBank(db, embedder, vectorBackend, learningBackend?)
     */
    constructor(db: IDatabaseConnection, embedder: EmbeddingService, vectorBackend?: VectorBackend, learningBackend?: LearningBackend);
    /**
     * Initialize reasoning patterns schema
     */
    private initializeSchema;
    /**
     * Store a reasoning pattern with embedding
     *
     * v1 (legacy): Stores in SQLite with pattern_embeddings table
     * v2 (VectorBackend): Stores metadata in SQLite, vectors in VectorBackend
     */
    storePattern(pattern: ReasoningPattern): Promise<number>;
    /**
     * Store pattern embedding
     */
    private storePatternEmbedding;
    /**
     * Search patterns by semantic similarity
     *
     * v1 (legacy): Uses SQLite with cosine similarity computation
     * v2 (VectorBackend): Uses high-performance vector search (8x faster)
     * v2 + GNN: Optionally enhances query with learned patterns
     */
    searchPatterns(query: PatternSearchQuery): Promise<ReasoningPattern[]>;
    /**
     * v2: Search using VectorBackend with optional GNN enhancement
     */
    private searchPatternsV2;
    /**
     * v1: Legacy search using SQLite (backward compatible)
     */
    private searchPatternsLegacy;
    /**
     * Hydrate search results with metadata from SQLite
     */
    private hydratePatterns;
    /**
     * Get embeddings for vector IDs (for GNN)
     */
    private getEmbeddingsForVectorIds;
    /**
     * Get pattern statistics
     */
    getPatternStats(): PatternStats;
    /**
     * Update pattern statistics after use
     */
    updatePatternStats(patternId: number, success: boolean, reward: number): void;
    /**
     * Record pattern outcome for GNN learning (v2 feature)
     *
     * Updates pattern stats and adds training sample to LearningBackend
     * for future GNN model improvements.
     *
     * @param patternId - Pattern ID to update
     * @param success - Whether the pattern was successful
     * @param reward - Optional reward value (default: 1 for success, 0 for failure)
     */
    recordOutcome(patternId: number, success: boolean, reward?: number): Promise<void>;
    /**
     * Train GNN model on collected samples (v2 feature)
     *
     * Trains the learning backend using accumulated pattern outcomes.
     * Requires LearningBackend to be configured.
     *
     * @param options - Training options (epochs, batchSize)
     * @returns Training results with epochs and final loss
     * @throws Error if LearningBackend not available
     */
    trainGNN(options?: {
        epochs?: number;
        batchSize?: number;
    }): Promise<{
        epochs: number;
        finalLoss: number;
    }>;
    /**
     * Get pattern by ID
     */
    getPattern(patternId: number): ReasoningPattern | null;
    /**
     * Delete pattern by ID
     */
    deletePattern(patternId: number): boolean;
    /**
     * Clear query cache
     */
    clearCache(): void;
    /**
     * Calculate cosine similarity between two vectors
     */
    private cosineSimilarity;
}
//# sourceMappingURL=ReasoningBank.d.ts.map