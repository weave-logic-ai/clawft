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
export class RuVectorLearning {
    gnnLayer; // RuvectorLayer from @ruvector/gnn
    config;
    initialized = false;
    differentiableSearch;
    hierarchicalForward;
    constructor(config) {
        this.config = {
            ...config,
            dropout: config.dropout ?? 0.1,
        };
    }
    /**
     * Initialize GNN layer with optional dependency handling
     */
    async initialize() {
        if (this.initialized)
            return;
        try {
            // Dynamic import with runtime property access
            const gnnModule = await import('@ruvector/gnn');
            this.gnnLayer = new gnnModule.RuvectorLayer(this.config.inputDim, this.config.hiddenDim, this.config.heads, this.config.dropout);
            this.differentiableSearch = gnnModule.differentiableSearch;
            this.hierarchicalForward = gnnModule.hierarchicalForward;
            this.initialized = true;
        }
        catch (error) {
            throw new Error(`GNN initialization failed. Please install: npm install @ruvector/gnn\n` +
                `Error: ${error.message}`);
        }
    }
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
    enhance(query, neighbors, weights) {
        this.ensureInitialized();
        if (neighbors.length === 0) {
            return query;
        }
        try {
            // Forward pass through GNN layer
            // Note: @ruvector/gnn v0.1.19+ expects Float32Array instead of number[]
            const result = this.gnnLayer.forward(query, // Already Float32Array
            neighbors, // Already Float32Array[]
            weights);
            // Result is already Float32Array in v0.1.19+
            return result instanceof Float32Array ? result : new Float32Array(result);
        }
        catch (error) {
            console.warn(`[RuVectorLearning] Enhancement failed: ${error.message}`);
            return query;
        }
    }
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
    search(query, candidates, options = {}) {
        this.ensureInitialized();
        const k = options.k ?? Math.min(5, candidates.length);
        const temperature = options.temperature ?? 1.0;
        try {
            // Note: @ruvector/gnn v0.1.19+ expects Float32Array instead of number[]
            const result = this.differentiableSearch(query, // Already Float32Array
            candidates, // Already Float32Array[]
            k, temperature);
            return result;
        }
        catch (error) {
            console.warn(`[RuVectorLearning] Differentiable search failed: ${error.message}`);
            // Fallback: return top-k indices with uniform weights
            return {
                indices: Array.from({ length: k }, (_, i) => i),
                weights: Array.from({ length: k }, () => 1.0 / k)
            };
        }
    }
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
    enhanceHierarchical(query, layerEmbeddings) {
        this.ensureInitialized();
        if (layerEmbeddings.length === 0) {
            return query;
        }
        try {
            // Serialize GNN layer for hierarchical processing
            const layerJson = this.gnnLayer.toJson();
            // Note: @ruvector/gnn v0.1.19+ expects Float32Array instead of number[]
            const result = this.hierarchicalForward(query, // Already Float32Array
            layerEmbeddings, // Already Float32Array[][]
            [layerJson] // Single layer for now
            );
            // Result is already Float32Array in v0.1.19+
            return result instanceof Float32Array ? result : new Float32Array(result);
        }
        catch (error) {
            console.warn(`[RuVectorLearning] Hierarchical enhancement failed: ${error.message}`);
            return query;
        }
    }
    /**
     * Serialize GNN layer to JSON
     *
     * Allows saving/loading the GNN layer configuration.
     *
     * @returns JSON string representation
     */
    toJson() {
        this.ensureInitialized();
        return this.gnnLayer.toJson();
    }
    /**
     * Create GNN layer from JSON
     *
     * @param json - JSON string from toJson()
     * @returns New RuVectorLearning instance
     */
    static async fromJson(json, config) {
        const learning = new RuVectorLearning(config);
        await learning.initialize();
        try {
            const gnnModule = await import('@ruvector/gnn');
            learning.gnnLayer = gnnModule.RuvectorLayer.fromJson(json);
            return learning;
        }
        catch (error) {
            throw new Error(`Failed to load GNN from JSON: ${error.message}`);
        }
    }
    /**
     * Get current state
     */
    getState() {
        return {
            initialized: this.initialized,
            config: this.config,
            hiddenDim: this.config.hiddenDim,
            heads: this.config.heads,
        };
    }
    /**
     * Ensure GNN is initialized before operations
     */
    ensureInitialized() {
        if (!this.initialized) {
            throw new Error('RuVectorLearning not initialized. Call initialize() first.');
        }
    }
}
//# sourceMappingURL=RuVectorLearning.js.map