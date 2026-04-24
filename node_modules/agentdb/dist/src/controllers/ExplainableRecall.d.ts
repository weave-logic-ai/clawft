/**
 * ExplainableRecall - Provenance and Justification for Memory Retrieval
 *
 * Every retrieval returns:
 * - Minimal hitting set of facts that justify the answer
 * - Merkle proof chain for provenance
 * - Policy compliance certificates
 *
 * Based on:
 * - Minimal hitting set algorithms
 * - Merkle tree provenance
 * - Explainable AI techniques
 *
 * v2.0.0-alpha.3 Features:
 * - GraphRoPE for hop-distance-aware graph queries (WASM)
 * - Rotary positional encoding based on graph structure
 * - Feature flag: ENABLE_GRAPH_ROPE (default: false)
 * - 100% backward compatible with fallback to standard retrieval
 */
type Database = any;
import { type GraphRoPEConfig } from '../services/AttentionService.js';
import { EmbeddingService } from './EmbeddingService.js';
/**
 * Configuration for ExplainableRecall
 */
export interface ExplainableRecallConfig {
    /** Enable GraphRoPE for hop-aware queries (default: false) */
    ENABLE_GRAPH_ROPE?: boolean;
    /** GraphRoPE configuration */
    graphRoPEConfig?: Partial<GraphRoPEConfig>;
}
export interface RecallCertificate {
    id: string;
    queryId: string;
    queryText: string;
    chunkIds: string[];
    chunkTypes: string[];
    minimalWhy: string[];
    redundancyRatio: number;
    completenessScore: number;
    merkleRoot: string;
    sourceHashes: string[];
    proofChain: MerkleProof[];
    policyProof?: string;
    policyVersion?: string;
    accessLevel: 'public' | 'internal' | 'confidential' | 'restricted';
    latencyMs?: number;
    metadata?: Record<string, any>;
}
export interface MerkleProof {
    hash: string;
    position: 'left' | 'right';
}
export interface JustificationPath {
    chunkId: string;
    chunkType: string;
    reason: 'semantic_match' | 'causal_link' | 'prerequisite' | 'constraint';
    necessityScore: number;
    pathElements: string[];
}
export interface ProvenanceSource {
    id?: number;
    sourceType: 'episode' | 'skill' | 'note' | 'fact' | 'external';
    sourceId: number;
    contentHash: string;
    parentHash?: string;
    derivedFrom?: string[];
    creator?: string;
    metadata?: Record<string, any>;
}
export declare class ExplainableRecall {
    private db;
    private attentionService?;
    private embedder?;
    private config;
    /**
     * Constructor supports both v1 (legacy) and v2 (with GraphRoPE) modes
     *
     * v1 mode: new ExplainableRecall(db)
     * v2 mode: new ExplainableRecall(db, embedder, config)
     */
    constructor(db: Database, embedder?: EmbeddingService, config?: ExplainableRecallConfig);
    /**
     * Create a recall certificate for a retrieval operation
     *
     * v2: Uses GraphRoPE if enabled for hop-distance-aware justification scoring
     * v1: Falls back to standard relevance-based justification
     */
    createCertificate(params: {
        queryId: string;
        queryText: string;
        chunks: Array<{
            id: string;
            type: string;
            content: string;
            relevance: number;
        }>;
        requirements: string[];
        accessLevel?: string;
        hopDistances?: number[][];
    }): Promise<RecallCertificate>;
    /**
     * Verify a recall certificate
     */
    verifyCertificate(certificateId: string): {
        valid: boolean;
        issues: string[];
    };
    /**
     * Get justification for why a chunk was included
     */
    getJustification(certificateId: string, chunkId: string): JustificationPath | null;
    /**
     * Get provenance lineage for a source
     */
    getProvenanceLineage(contentHash: string): ProvenanceSource[];
    /**
     * Trace provenance lineage for a certificate
     * Returns full provenance chain from certificate to original sources
     */
    traceProvenance(certificateId: string): {
        certificate: RecallCertificate;
        sources: Map<string, ProvenanceSource[]>;
        graph: {
            nodes: Array<{
                id: string;
                type: string;
                label: string;
            }>;
            edges: Array<{
                from: string;
                to: string;
                type: string;
            }>;
        };
    };
    /**
     * Audit certificate access
     */
    auditCertificate(certificateId: string): {
        certificate: RecallCertificate;
        justifications: JustificationPath[];
        provenance: Map<string, ProvenanceSource[]>;
        quality: {
            completeness: number;
            redundancy: number;
            avgNecessity: number;
        };
    };
    /**
     * Compute minimal hitting set using greedy algorithm
     * A hitting set contains at least one element from each requirement
     */
    private computeMinimalHittingSet;
    /**
     * Calculate completeness score
     */
    private calculateCompleteness;
    /**
     * Get or create provenance record
     */
    private getOrCreateProvenance;
    private _episodeStmt?;
    private _skillStmt?;
    private _noteStmt?;
    private _factStmt?;
    /**
     * Get content hash for a memory
     */
    private getContentHash;
    /**
     * Build Merkle tree from hashes
     */
    private buildMerkleTree;
    /**
     * Get Merkle proof for a leaf
     */
    private getMerkleProof;
    /**
     * Generate certificate ID
     */
    private generateCertificateId;
    /**
     * Store justification paths
     */
    private storeJustificationPaths;
    /**
     * Determine reason for inclusion
     */
    private determineReason;
}
export {};
//# sourceMappingURL=ExplainableRecall.d.ts.map