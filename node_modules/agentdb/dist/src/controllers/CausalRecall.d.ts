/**
 * CausalRecall - Utility-Based Reranking + Certificate Issuer
 *
 * Combines:
 * 1. Vector similarity search
 * 2. Causal uplift from CausalMemoryGraph
 * 3. Utility-based reranking: U = α*similarity + β*uplift − γ*latencyCost
 * 4. Automatic certificate issuance via ExplainableRecall
 *
 * This is the main entry point for production retrieval with:
 * - Causal-aware ranking
 * - Explainable provenance
 * - Policy compliance
 */
type Database = any;
import { RecallCertificate } from './ExplainableRecall.js';
import { EmbeddingService } from './EmbeddingService.js';
import type { VectorBackend } from '../backends/VectorBackend.js';
export interface RerankConfig {
    alpha: number;
    beta: number;
    gamma: number;
    minConfidence?: number;
}
export interface RerankCandidate {
    id: string;
    type: 'episode' | 'skill' | 'note' | 'fact';
    content: string;
    similarity: number;
    uplift?: number;
    causalConfidence?: number;
    latencyMs?: number;
    utilityScore: number;
    rank: number;
}
export interface CausalRecallResult {
    candidates: RerankCandidate[];
    certificate: RecallCertificate;
    queryId: string;
    totalLatencyMs: number;
    metrics: {
        vectorSearchMs: number;
        causalLookupMs: number;
        rerankMs: number;
        certificateMs: number;
    };
}
export declare class CausalRecall {
    private config;
    private db;
    private causalGraph;
    private explainableRecall;
    private embedder;
    private vectorBackend?;
    constructor(db: Database, embedder: EmbeddingService, vectorBackend?: VectorBackend, config?: RerankConfig);
    /**
     * Main recall function with utility-based reranking and certificate issuance
     *
     * @param queryId Unique query identifier
     * @param queryText Natural language query
     * @param k Number of results to return (default: 12)
     * @param requirements Optional list of requirements for completeness checking
     * @param accessLevel Security access level for certificate
     * @returns Reranked results with certificate
     */
    recall(queryId: string, queryText: string, k?: number, requirements?: string[], accessLevel?: 'public' | 'internal' | 'confidential' | 'restricted'): Promise<CausalRecallResult>;
    /**
     * Vector similarity search using cosine similarity
     */
    private vectorSearch;
    /**
     * Load causal edges for candidates
     */
    private loadCausalEdges;
    /**
     * Rerank by utility: U = α*similarity + β*uplift − γ*latencyCost
     */
    private rerankByUtility;
    /**
     * Issue certificate for the retrieval
     */
    private issueCertificate;
    /**
     * Extract requirements from query text (simple keyword extraction)
     */
    private extractRequirements;
    /**
     * Deserialize embedding from Buffer
     */
    private deserializeEmbedding;
    /**
     * Cosine similarity between two vectors
     */
    private cosineSimilarity;
    /**
     * Batch recall for multiple queries
     */
    batchRecall(queries: Array<{
        queryId: string;
        queryText: string;
        k?: number;
    }>, requirements?: string[], accessLevel?: 'public' | 'internal' | 'confidential' | 'restricted'): Promise<CausalRecallResult[]>;
    /**
     * Get recall statistics
     */
    getStats(): {
        totalCausalEdges: number;
        totalCertificates: number;
        avgRedundancyRatio: number;
        avgCompletenessScore: number;
    };
    /**
     * Update rerank configuration
     */
    updateConfig(config: Partial<RerankConfig>): void;
    /**
     * Search for memories with semantic similarity and causal utility ranking
     *
     * @param params Search parameters
     * @returns Array of ranked search results with similarity and causal uplift scores
     */
    search(params: {
        query: string;
        k?: number;
        includeEvidence?: boolean;
        alpha?: number;
        beta?: number;
        gamma?: number;
    }): Promise<Array<{
        id: number;
        type: string;
        content: string;
        similarity: number;
        causalUplift: number;
        utilityScore: number;
    }>>;
}
export {};
//# sourceMappingURL=CausalRecall.d.ts.map