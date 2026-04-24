/**
 * RuVectorLearning - GNN-Enhanced Vector Search
 *
 * Integrates Graph Neural Networks for query enhancement using @ruvector/gnn.
 * Requires optional @ruvector/gnn package.
 *
 * Features:
 * - Query enhancement using neighbor context with multi-head attention
 * - Differentiable search with soft weights
 * - Hierarchical forward pass through GNN layers
 * - Graceful degradation when GNN not available
 *
 * Note: @ruvector/gnn provides stateless GNN layers (inference only).
 * Training is handled separately by the consuming application.
 */
export interface LearningConfig {
    inputDim: number;
    hiddenDim: number;
    heads: number;
    dropout?: number;
}
export interface EnhancementOptions {
    temperature?: number;
    k?: number;
}
export declare class RuVectorLearning {
    private gnnLayer;
    private config;
    private initialized;
    private differentiableSearch;
    private hierarchicalForward;
    constructor(config: LearningConfig);
    /**
     * Initialize GNN layer with optional dependency handling
     */
    initialize(): Promise<void>;
    /**
     * Enhance query embedding using neighbor context
     *
     * Uses Graph Attention Network to aggregate information from
     * nearest neighbors, weighted by their relevance scores.
     *
     * @param query - Query embedding to enhance
     * @param neighbors - Neighbor embeddings
     * @param weights - Edge weights (relevance scores)
     * @returns Enhanced query embedding
     */
    enhance(query: Float32Array, neighbors: Float32Array[], weights: number[]): Float32Array;
    /**
     * Differentiable search with soft attention
     *
     * Uses soft attention mechanism instead of hard top-k selection.
     * Returns indices and weights that can be used for gradient-based optimization.
     *
     * @param query - Query embedding
     * @param candidates - Candidate embeddings
     * @param options - Search options
     * @returns Search result with indices and soft weights
     */
    search(query: Float32Array, candidates: Float32Array[], options?: EnhancementOptions): {
        indices: number[];
        weights: number[];
    };
    /**
     * Hierarchical forward pass through multiple GNN layers
     *
     * Used for HNSW-style hierarchical search where embeddings
     * are organized by graph layers.
     *
     * @param query - Query embedding
     * @param layerEmbeddings - Embeddings organized by layer
     * @returns Final enhanced embedding
     */
    enhanceHierarchical(query: Float32Array, layerEmbeddings: Float32Array[][]): Float32Array;
    /**
     * Serialize GNN layer to JSON
     *
     * Allows saving/loading the GNN layer configuration.
     *
     * @returns JSON string representation
     */
    toJson(): string;
    /**
     * Create GNN layer from JSON
     *
     * @param json - JSON string from toJson()
     * @returns New RuVectorLearning instance
     */
    static fromJson(json: string, config: LearningConfig): Promise<RuVectorLearning>;
    /**
     * Get current state
     */
    getState(): {
        initialized: boolean;
        config: LearningConfig;
        hiddenDim: number;
        heads: number;
    };
    /**
     * Ensure GNN is initialized before operations
     */
    private ensureInitialized;
}
//# sourceMappingURL=RuVectorLearning.d.ts.map