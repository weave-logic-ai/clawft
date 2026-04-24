/**
 * TypeScript type definitions for AgentDB wrapper
 *
 * Provides clean, typed interfaces for all AgentDB operations
 * following v2.0.0-alpha.2.11 specification
 */
/**
 * Vector search metrics supported by AgentDB
 */
export type DistanceMetric = 'cosine' | 'euclidean' | 'dot' | 'manhattan';
/**
 * HNSW index configuration
 */
export interface HNSWConfig {
    /** Maximum number of connections per element (default: 16) */
    M?: number;
    /** Size of dynamic candidate list during construction (default: 200) */
    efConstruction?: number;
    /** Size of dynamic candidate list during search (default: 100) */
    efSearch?: number;
}
/**
 * Vector embedding with metadata
 */
export interface VectorEntry {
    /** Unique identifier for the vector */
    id: string;
    /** Vector embedding (Float32Array for performance) */
    vector: Float32Array;
    /** Optional metadata for filtering and retrieval */
    metadata?: Record<string, any>;
    /** Timestamp of insertion */
    timestamp?: number;
}
/**
 * Vector search result
 */
export interface VectorSearchResult {
    /** Vector entry ID */
    id: string;
    /** Similarity score (higher = more similar) */
    score: number;
    /** Metadata associated with the vector */
    metadata?: Record<string, any>;
    /** Optional vector data if requested */
    vector?: Float32Array;
}
/**
 * Vector search options
 */
export interface VectorSearchOptions {
    /** Number of results to return (default: 10) */
    k?: number;
    /** Distance metric (default: 'cosine') */
    metric?: DistanceMetric;
    /** Metadata filters (key-value pairs) */
    filter?: Record<string, any>;
    /** Include vector data in results (default: false) */
    includeVectors?: boolean;
    /** HNSW-specific search parameters */
    hnswParams?: {
        efSearch?: number;
    };
}
/**
 * Memory operation types
 */
export type MemoryOperation = 'insert' | 'search' | 'update' | 'delete' | 'get';
/**
 * Memory insert options
 */
export interface MemoryInsertOptions {
    /** Vector embedding */
    vector: Float32Array;
    /** Metadata for the memory */
    metadata?: Record<string, any>;
    /** Optional custom ID (auto-generated if not provided) */
    id?: string;
    /** Namespace for organizing memories */
    namespace?: string;
}
/**
 * Memory update options
 */
export interface MemoryUpdateOptions {
    /** Vector ID to update */
    id: string;
    /** New vector embedding (optional) */
    vector?: Float32Array;
    /** New metadata (merged with existing) */
    metadata?: Record<string, any>;
    /** Namespace */
    namespace?: string;
}
/**
 * Memory delete options
 */
export interface MemoryDeleteOptions {
    /** Vector ID to delete */
    id: string;
    /** Namespace */
    namespace?: string;
}
/**
 * Memory get options
 */
export interface MemoryGetOptions {
    /** Vector ID to retrieve */
    id: string;
    /** Namespace */
    namespace?: string;
    /** Include vector data (default: true) */
    includeVector?: boolean;
}
/**
 * Attention mechanism types
 */
export type AttentionType = 'multi-head' | 'flash' | 'linear' | 'hyperbolic' | 'moe' | 'graph-rope';
/**
 * Attention configuration
 */
export interface AttentionConfig {
    /** Attention type */
    type?: AttentionType;
    /** Number of attention heads (default: 8) */
    numHeads?: number;
    /** Head dimension (default: 64) */
    headDim?: number;
    /** Embedding dimension (inferred from vector dimension) */
    embedDim?: number;
    /** Dropout rate (default: 0.1) */
    dropout?: number;
    /** Hyperbolic curvature for hyperbolic attention */
    curvature?: number;
    /** Number of experts for MoE attention */
    numExperts?: number;
    /** Top-k experts to activate (default: 2) */
    topK?: number;
}
/**
 * GNN configuration
 */
export interface GNNConfig {
    /** Enable GNN query refinement (default: false) */
    enabled?: boolean;
    /** Input dimension (matches vector dimension) */
    inputDim?: number;
    /** Hidden layer dimension (default: 256) */
    hiddenDim?: number;
    /** Number of GNN layers (default: 3) */
    numLayers?: number;
    /** Number of attention heads per layer (default: 8) */
    numHeads?: number;
    /** Aggregation method */
    aggregation?: 'mean' | 'sum' | 'max' | 'attention';
}
/**
 * Graph context for GNN operations
 */
export interface GraphContext {
    /** Node features (embeddings) */
    nodes: Float32Array[];
    /** Edge list [source, target] pairs */
    edges: [number, number][];
    /** Optional edge weights */
    edgeWeights?: number[];
    /** Optional node labels */
    nodeLabels?: string[];
}
/**
 * AgentDB configuration
 */
export interface AgentDBConfig {
    /** Database file path (default: ':memory:') */
    dbPath?: string;
    /** Namespace for organizing data */
    namespace?: string;
    /** Vector dimension (default: 384) */
    dimension?: number;
    /** HNSW index configuration */
    hnswConfig?: HNSWConfig;
    /** Enable attention mechanisms (default: false) */
    enableAttention?: boolean;
    /** Attention configuration */
    attentionConfig?: AttentionConfig;
    /** Enable GNN query refinement (default: false) */
    enableGNN?: boolean;
    /** GNN configuration */
    gnnConfig?: GNNConfig;
    /** Enable auto-initialization (default: true) */
    autoInit?: boolean;
}
/**
 * Statistics about the AgentDB instance
 */
export interface AgentDBStats {
    /** Total number of vectors */
    vectorCount: number;
    /** Vector dimension */
    dimension: number;
    /** Database size in bytes */
    databaseSize: number;
    /** HNSW index statistics */
    hnswStats?: {
        M: number;
        efConstruction: number;
        efSearch: number;
        levels: number;
    };
    /** Memory usage in bytes */
    memoryUsage?: number;
    /** Index build time in ms */
    indexBuildTime?: number;
}
/**
 * Batch insert result
 */
export interface BatchInsertResult {
    /** Number of successfully inserted vectors */
    inserted: number;
    /** Failed insertions with errors */
    failed: Array<{
        index: number;
        id?: string;
        error: string;
    }>;
    /** Total time in milliseconds */
    duration: number;
}
/**
 * Error types for AgentDB operations
 */
export declare class AgentDBError extends Error {
    readonly code: string;
    readonly operation: MemoryOperation;
    readonly details?: any;
    constructor(message: string, code: string, operation: MemoryOperation, details?: any);
}
/**
 * Validation error for invalid inputs
 */
export declare class ValidationError extends AgentDBError {
    constructor(message: string, details?: any);
}
/**
 * Database error for storage issues
 */
export declare class DatabaseError extends AgentDBError {
    constructor(message: string, operation: MemoryOperation, details?: any);
}
/**
 * Index error for HNSW operations
 */
export declare class IndexError extends AgentDBError {
    constructor(message: string, operation: MemoryOperation, details?: any);
}
/**
 * Attention result with performance metrics
 */
export interface AttentionResult {
    /** Output tensor */
    output: Float32Array;
    /** Attention mechanism used */
    mechanism: AttentionType;
    /** Runtime environment (napi or wasm) */
    runtime: 'napi' | 'wasm' | 'js';
    /** Execution time in milliseconds */
    executionTimeMs: number;
    /** Optional attention weights */
    attentionWeights?: Float32Array;
    /** Memory usage in bytes */
    memoryUsage?: number;
}
/**
 * GNN refinement result
 */
export interface GNNRefinementResult {
    /** Refined results */
    results: VectorSearchResult[];
    /** Original recall */
    originalRecall: number;
    /** Improved recall after GNN */
    improvedRecall: number;
    /** Recall improvement percentage */
    improvementPercent: number;
    /** GNN execution time */
    executionTimeMs: number;
}
/**
 * Vector search options with advanced features
 */
export interface AdvancedSearchOptions extends VectorSearchOptions {
    /** Use attention-based reranking */
    useAttention?: boolean;
    /** Attention mechanism to use */
    attentionMechanism?: AttentionType;
    /** Use GNN query refinement */
    useGNN?: boolean;
    /** Graph context for GNN */
    graphContext?: GraphContext;
    /** Include performance metrics */
    includeMetrics?: boolean;
}
//# sourceMappingURL=agentdb.d.ts.map