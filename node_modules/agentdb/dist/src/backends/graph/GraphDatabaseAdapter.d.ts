/**
 * RuVector Graph Database Adapter - Primary Database for AgentDB v2
 *
 * Replaces SQLite with RuVector's graph database for:
 * - Episodes as nodes with vector embeddings
 * - Skills as nodes with code embeddings
 * - Causal relationships as hyperedges
 * - Cypher queries instead of SQL
 *
 * Features:
 * - 10x faster than WASM SQLite
 * - ACID transactions with persistence
 * - Vector similarity search integrated
 * - Hypergraph support for complex relationships
 * - Neo4j-compatible Cypher syntax
 */
type JsNode = {
    id: string;
    embedding: Float32Array;
    labels?: Array<string>;
    properties?: Record<string, string>;
};
type JsEdge = {
    from: string;
    to: string;
    description: string;
    embedding: Float32Array;
    confidence?: number;
    metadata?: Record<string, string>;
};
type JsQueryResult = {
    nodes: Array<any>;
    edges: Array<any>;
    stats?: any;
};
export interface GraphDatabaseConfig {
    storagePath: string;
    dimensions?: number;
    distanceMetric?: 'Cosine' | 'Euclidean' | 'DotProduct' | 'Manhattan';
}
export interface EpisodeNode {
    id: string;
    sessionId: string;
    task: string;
    reward: number;
    success: boolean;
    input?: string;
    output?: string;
    critique?: string;
    createdAt: number;
    tokensUsed?: number;
    latencyMs?: number;
}
export interface SkillNode {
    id: string;
    name: string;
    description: string;
    code: string;
    usageCount: number;
    avgReward: number;
    createdAt: number;
    updatedAt: number;
    tags?: string;
}
export interface CausalEdge {
    from: string;
    to: string;
    mechanism: string;
    uplift: number;
    confidence: number;
    sampleSize: number;
}
/**
 * Graph Database Adapter for AgentDB
 *
 * This replaces SQL.js as the primary database, using RuVector's graph DB
 * with Cypher queries, hyperedges, and integrated vector search.
 */
export declare class GraphDatabaseAdapter {
    private db;
    private config;
    private embedder;
    constructor(config: GraphDatabaseConfig, embedder: any);
    /**
     * Initialize graph database (create new or open existing)
     */
    initialize(): Promise<void>;
    /**
     * Store an episode as a graph node
     */
    storeEpisode(episode: EpisodeNode, embedding: Float32Array): Promise<string>;
    /**
     * Store a skill as a graph node
     */
    storeSkill(skill: SkillNode, embedding: Float32Array): Promise<string>;
    /**
     * Create a causal relationship edge
     */
    createCausalEdge(edge: CausalEdge, embedding: Float32Array): Promise<string>;
    /**
     * Query using Cypher syntax
     *
     * Examples:
     * - MATCH (e:Episode) WHERE e.success = 'true' RETURN e
     * - MATCH (s:Skill) RETURN s ORDER BY s.avgReward DESC LIMIT 10
     * - MATCH (e1:Episode)-[r]->(e2:Episode) RETURN e1, r, e2
     */
    query(cypher: string): Promise<JsQueryResult>;
    /**
     * Search for similar episodes by embedding
     */
    searchSimilarEpisodes(embedding: Float32Array, k?: number): Promise<any[]>;
    /**
     * Search for similar skills by embedding
     */
    searchSkills(embedding: Float32Array, k?: number): Promise<SkillNode[]>;
    /**
     * Generic createNode method for graph traversal scenarios
     */
    createNode(node: JsNode): Promise<string>;
    /**
     * Generic createEdge method for graph traversal scenarios
     */
    createEdge(edge: JsEdge): Promise<void>;
    /**
     * Get graph statistics
     */
    getStats(): Promise<any>;
    /**
     * Begin transaction
     */
    beginTransaction(): Promise<string>;
    /**
     * Commit transaction
     */
    commitTransaction(txId: string): Promise<void>;
    /**
     * Rollback transaction
     */
    rollbackTransaction(txId: string): Promise<void>;
    /**
     * Batch insert nodes and edges
     */
    batchInsert(nodes: JsNode[], edges: JsEdge[]): Promise<any>;
    /**
     * Close database
     */
    close(): void;
}
export {};
//# sourceMappingURL=GraphDatabaseAdapter.d.ts.map