/**
 * Nightly Learner - Automated Causal Discovery and Consolidation
 *
 * Runs as a background job to:
 * 1. Discover new causal edges from episode patterns
 * 2. Run A/B experiments on promising hypotheses
 * 3. Calculate uplift for completed experiments
 * 4. Prune low-confidence edges
 * 5. Update rerank weights based on performance
 *
 * Based on doubly robust learner:
 * τ̂(x) = μ1(x) − μ0(x) + [a*(y−μ1(x)) / e(x)] − [(1−a)*(y−μ0(x)) / (1−e(x))]
 *
 * v2.0.0-alpha.3 Features:
 * - FlashAttention for memory-efficient episodic consolidation
 * - Block-wise computation for large episode buffers
 * - Feature flag: ENABLE_FLASH_CONSOLIDATION (default: false)
 * - 100% backward compatible with fallback to standard consolidation
 */
type Database = any;
import { CausalEdge } from './CausalMemoryGraph.js';
import { EmbeddingService } from './EmbeddingService.js';
import { type FlashAttentionConfig } from '../services/AttentionService.js';
export interface LearnerConfig {
    minSimilarity: number;
    minSampleSize: number;
    confidenceThreshold: number;
    upliftThreshold: number;
    pruneOldEdges: boolean;
    edgeMaxAgeDays: number;
    autoExperiments: boolean;
    experimentBudget: number;
    /** Enable FlashAttention for consolidation (default: false) */
    ENABLE_FLASH_CONSOLIDATION?: boolean;
    /** FlashAttention configuration */
    flashConfig?: Partial<FlashAttentionConfig>;
}
export interface LearnerReport {
    timestamp: number;
    executionTimeMs: number;
    edgesDiscovered: number;
    edgesPruned: number;
    experimentsCompleted: number;
    experimentsCreated: number;
    avgUplift: number;
    avgConfidence: number;
    recommendations: string[];
}
export declare class NightlyLearner {
    private config;
    private db;
    private causalGraph;
    private reflexion;
    private skillLibrary;
    private embedder;
    private attentionService?;
    constructor(db: Database, embedder: EmbeddingService, config?: LearnerConfig);
    /**
     * Main learning job - runs all discovery and consolidation tasks
     */
    run(): Promise<LearnerReport>;
    /**
     * Discover causal edges using doubly robust learner
     *
     * τ̂(x) = μ1(x) − μ0(x) + [a*(y−μ1(x)) / e(x)] − [(1−a)*(y−μ0(x)) / (1−e(x))]
     *
     * Where:
     * - μ1(x) = outcome model for treatment
     * - μ0(x) = outcome model for control
     * - e(x) = propensity score (probability of treatment)
     * - a = treatment indicator
     * - y = observed outcome
     *
     * v2: Uses FlashAttention for memory-efficient consolidation if enabled
     */
    discover(config: {
        minAttempts?: number;
        minSuccessRate?: number;
        minConfidence?: number;
        dryRun?: boolean;
    }): Promise<CausalEdge[]>;
    /**
     * Consolidate episodic memories using FlashAttention (v2 feature)
     *
     * Processes large episode buffers efficiently using block-wise computation.
     * Identifies patterns and relationships across episodes for causal edge discovery.
     *
     * @param sessionId - Session to consolidate (optional, processes all if not provided)
     * @returns Number of edges discovered through consolidation
     */
    consolidateEpisodes(sessionId?: string): Promise<{
        edgesDiscovered: number;
        episodesProcessed: number;
        metrics?: {
            computeTimeMs: number;
            peakMemoryMB: number;
            blocksProcessed: number;
        };
    }>;
    /**
     * Helper: Cosine similarity between two vectors
     */
    private cosineSimilarity;
    private discoverCausalEdges;
    /**
     * Calculate propensity score e(x) - probability of treatment given context
     */
    private calculatePropensity;
    /**
     * Calculate outcome model μ(x) - expected outcome given treatment status
     */
    private calculateOutcomeModel;
    /**
     * Get sample size for a task type
     */
    private getSampleSize;
    /**
     * Calculate confidence based on sample size and effect size
     */
    private calculateConfidence;
    /**
     * Complete running A/B experiments and calculate uplift
     */
    private completeExperiments;
    /**
     * Create new A/B experiments for promising hypotheses
     */
    private createExperiments;
    /**
     * Prune old or low-confidence edges
     */
    private pruneEdges;
    /**
     * Calculate overall statistics
     */
    private calculateStats;
    /**
     * Generate recommendations based on learning results
     */
    private generateRecommendations;
    /**
     * Print report to console
     */
    private printReport;
    /**
     * Update learner configuration
     */
    updateConfig(config: Partial<LearnerConfig>): void;
}
export {};
//# sourceMappingURL=NightlyLearner.d.ts.map