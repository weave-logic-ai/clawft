/**
 * HNSW (Hierarchical Navigable Small World) Index for Browser
 *
 * JavaScript implementation of HNSW algorithm for fast approximate nearest neighbor search.
 * Achieves O(log n) search complexity vs O(n) for linear scan.
 *
 * Features:
 * - Multi-layer graph structure
 * - Probabilistic layer assignment
 * - Greedy search algorithm
 * - Dynamic insertion
 * - Configurable M (connections per node)
 * - Configurable efConstruction and efSearch
 *
 * Performance:
 * - 10-20x faster than linear scan (vs 150x for native HNSW)
 * - Memory: ~16 bytes per edge + vector storage
 * - Suitable for datasets up to 100K vectors in browser
 */
export interface HNSWConfig {
    dimension: number;
    M: number;
    efConstruction: number;
    efSearch: number;
    ml: number;
    maxLayers: number;
    distanceFunction?: 'cosine' | 'euclidean' | 'manhattan';
}
export interface HNSWNode {
    id: number;
    vector: Float32Array;
    level: number;
    connections: Map<number, number[]>;
}
export interface SearchResult {
    id: number;
    distance: number;
    vector: Float32Array;
}
export declare class HNSWIndex {
    private config;
    private nodes;
    private entryPoint;
    private currentId;
    private ml;
    constructor(config?: Partial<HNSWConfig>);
    /**
     * Add vector to index
     */
    add(vector: Float32Array, id?: number): number;
    /**
     * Search for k nearest neighbors
     */
    search(query: Float32Array, k: number, ef?: number): SearchResult[];
    /**
     * Search at specific layer
     */
    private searchLayer;
    /**
     * Select best neighbors using heuristic
     */
    private selectNeighbors;
    /**
     * Connect two nodes at layer
     */
    private connect;
    /**
     * Random level assignment
     */
    private randomLevel;
    /**
     * Distance function
     */
    private distance;
    private cosineSimilarity;
    private euclideanDistance;
    private manhattanDistance;
    /**
     * Get index statistics
     */
    getStats(): {
        numNodes: number;
        numLayers: number;
        avgConnections: number;
        entryPointLevel: number;
        memoryBytes: number;
    };
    /**
     * Export index for persistence
     */
    export(): string;
    /**
     * Import index from JSON
     */
    import(json: string): void;
    /**
     * Clear index
     */
    clear(): void;
    /**
     * Get number of nodes
     */
    size(): number;
}
/**
 * Helper function to create HNSW index with default settings
 */
export declare function createHNSW(dimension: number): HNSWIndex;
/**
 * Helper function to create fast HNSW (lower quality, faster build)
 */
export declare function createFastHNSW(dimension: number): HNSWIndex;
/**
 * Helper function to create accurate HNSW (higher quality, slower build)
 */
export declare function createAccurateHNSW(dimension: number): HNSWIndex;
//# sourceMappingURL=HNSWIndex.d.ts.map