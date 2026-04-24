/**
 * Advanced Features for AgentDB Browser
 *
 * Includes:
 * - GNN (Graph Neural Networks) - Graph attention and message passing
 * - MMR (Maximal Marginal Relevance) - Diversity ranking
 * - SVD (Singular Value Decomposition) - Tensor compression
 * - Batch operations and utilities
 */
export interface GNNNode {
    id: number;
    features: Float32Array;
    neighbors: number[];
}
export interface GNNEdge {
    from: number;
    to: number;
    weight: number;
}
export interface GNNConfig {
    hiddenDim: number;
    numHeads: number;
    dropout: number;
    learningRate: number;
    attentionType: 'gat' | 'gcn' | 'sage';
}
/**
 * Graph Neural Network with attention mechanism
 */
export declare class GraphNeuralNetwork {
    private config;
    private nodes;
    private edges;
    private attentionWeights;
    constructor(config?: Partial<GNNConfig>);
    /**
     * Add node to graph
     */
    addNode(id: number, features: Float32Array): void;
    /**
     * Add edge to graph
     */
    addEdge(from: number, to: number, weight?: number): void;
    /**
     * Graph Attention Network (GAT) message passing
     */
    graphAttention(nodeId: number): Float32Array;
    /**
     * Compute attention score between two nodes
     */
    private computeAttentionScore;
    /**
     * Message passing for all nodes
     */
    messagePass(): Map<number, Float32Array>;
    /**
     * Update node features after message passing
     */
    update(newFeatures: Map<number, Float32Array>): void;
    /**
     * Compute graph embeddings for query enhancement
     */
    computeGraphEmbedding(nodeId: number, hops?: number): Float32Array;
    /**
     * Get statistics
     */
    getStats(): {
        numNodes: number;
        numEdges: number;
        avgDegree: number;
        config: GNNConfig;
    };
}
export interface MMRConfig {
    lambda: number;
    metric: 'cosine' | 'euclidean';
}
/**
 * Maximal Marginal Relevance for diversity ranking
 */
export declare class MaximalMarginalRelevance {
    private config;
    constructor(config?: Partial<MMRConfig>);
    /**
     * Rerank results for diversity
     * @param query Query vector
     * @param candidates Candidate vectors with scores
     * @param k Number of results to return
     * @returns Reranked indices
     */
    rerank(query: Float32Array, candidates: Array<{
        id: number;
        vector: Float32Array;
        score: number;
    }>, k: number): number[];
    /**
     * Similarity computation
     */
    private similarity;
    private cosineSimilarity;
    private euclideanDistance;
    /**
     * Set lambda (relevance vs diversity trade-off)
     */
    setLambda(lambda: number): void;
}
/**
 * Simple SVD implementation for dimension reduction
 */
export declare class TensorCompression {
    /**
     * Reduce dimensionality using truncated SVD
     * @param vectors Array of vectors to compress
     * @param targetDim Target dimension
     * @returns Compressed vectors
     */
    static compress(vectors: Float32Array[], targetDim: number): Float32Array[];
    /**
     * Compute mean vector
     */
    private static computeMean;
    /**
     * Compute covariance matrix
     */
    private static computeCovariance;
    /**
     * Power iteration for computing top eigenvectors
     */
    private static powerIteration;
}
/**
 * Efficient batch processing utilities
 */
export declare class BatchProcessor {
    /**
     * Batch cosine similarity computation
     */
    static batchCosineSimilarity(query: Float32Array, vectors: Float32Array[]): Float32Array;
    /**
     * Batch vector normalization
     */
    static batchNormalize(vectors: Float32Array[]): Float32Array[];
}
//# sourceMappingURL=AdvancedFeatures.d.ts.map