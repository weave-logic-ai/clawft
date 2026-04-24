/**
 * RuVector-Enhanced Agent Booster v2
 *
 * Full upgrade with all RuVector capabilities:
 * - Semantic fuzzy matching (cosine similarity on embeddings)
 * - ONNX embeddings for semantic code understanding
 * - Parallel batch apply for multi-file edits
 * - Context-aware prefetch (predict likely edits)
 * - Error pattern learning (learn what NOT to do)
 * - TensorCompress for 10x more patterns in memory
 * - SONA continual learning with EWC++
 * - GNN differentiable search
 *
 * Performance targets:
 * - Exact match: 0ms
 * - Fuzzy match: 1-5ms
 * - Cache miss: 650ms (agent-booster)
 * - Pattern capacity: 100,000+ with compression
 */
export interface EnhancedEditRequest {
    code: string;
    edit: string;
    language: string;
    filePath?: string;
    context?: string;
}
export interface EnhancedEditResult {
    output: string;
    success: boolean;
    latency: number;
    confidence: number;
    strategy: 'exact_cache' | 'fuzzy_match' | 'gnn_match' | 'agent_booster' | 'fallback' | 'error_avoided';
    cacheHit: boolean;
    learned: boolean;
    patternId?: string;
    similarPatterns?: number;
    fuzzyScore?: number;
}
export interface LearnedPattern {
    id: string;
    codeHash: string;
    editHash: string;
    language: string;
    embedding: number[];
    confidence: number;
    successCount: number;
    failureCount: number;
    avgLatency: number;
    lastUsed: number;
    output?: string;
    codeNormalized?: string;
    editType?: string;
    compressed?: boolean;
    accessCount: number;
    createdAt: number;
    compressionTier?: 'none' | 'half' | 'pq8' | 'pq4' | 'binary';
}
export interface ErrorPattern {
    pattern: string;
    errorType: string;
    suggestedFix: string;
    occurrences: number;
    lastSeen: number;
}
export interface BoosterStats {
    totalEdits: number;
    cacheHits: number;
    fuzzyHits: number;
    gnnHits: number;
    cacheMisses: number;
    avgLatency: number;
    avgConfidence: number;
    patternsLearned: number;
    errorPatternsLearned: number;
    sonaUpdates: number;
    gnnSearches: number;
    hitRate: string;
    confidenceImprovement: string;
    compressionRatio: string;
    onnxEnabled: boolean;
    tierDistribution: {
        hot: number;
        warm: number;
        cool: number;
        cold: number;
        archive: number;
    };
    totalPatternAccesses: number;
    memorySavings: string;
}
export interface PrefetchResult {
    file: string;
    likelyEdits: string[];
    confidence: number;
}
/**
 * RuVector-Enhanced Agent Booster v2
 */
export declare class EnhancedAgentBooster {
    private intelligence;
    private patterns;
    private patternEmbeddings;
    private patternIds;
    private errorPatterns;
    private coEditGraph;
    private tensorCompress;
    private stats;
    private storagePath;
    private initialized;
    private onnxReady;
    private enableOnnx;
    private fuzzyThreshold;
    private maxPatterns;
    private totalPatternAccesses;
    private lastRecompressionTime;
    private recompressionInterval;
    constructor(options?: {
        storagePath?: string;
        enableOnnx?: boolean;
        enableSona?: boolean;
        maxPatterns?: number;
        fuzzyThreshold?: number;
    });
    /**
     * Initialize the enhanced booster (load patterns, init ONNX)
     */
    init(): Promise<void>;
    /**
     * Initialize ONNX in background (non-blocking)
     */
    private initOnnxAsync;
    /**
     * Calculate access frequency for a pattern
     * Based on pattern access count relative to total accesses
     *
     * Compression Tiers:
     * | Data Type             | Access Freq | Compression | Memory Savings |
     * |-----------------------|-------------|-------------|----------------|
     * | Hot patterns (recent) | >0.8        | none        | 0%             |
     * | Warm patterns         | >0.4        | half        | 50%            |
     * | Cool patterns         | >0.1        | pq8         | 87.5%          |
     * | Cold patterns         | >0.01       | pq4         | 93.75%         |
     * | Archive               | ≤0.01       | binary      | 96.9%          |
     */
    private calculateAccessFrequency;
    /**
     * Get compression tier based on access frequency
     */
    private getCompressionTier;
    /**
     * Apply tiered compression to an embedding
     */
    private applyTieredCompression;
    /**
     * Periodically recompress patterns based on updated access frequencies
     */
    private recompressPatterns;
    /**
     * Apply code edit with full RuVector enhancement
     */
    apply(request: EnhancedEditRequest): Promise<EnhancedEditResult>;
    /**
     * Apply multiple edits in parallel
     */
    applyBatch(requests: EnhancedEditRequest[], maxConcurrency?: number): Promise<EnhancedEditResult[]>;
    /**
     * Prefetch likely edits based on context
     */
    prefetch(filePath: string): Promise<PrefetchResult>;
    /**
     * Check for exact pattern match
     */
    private checkExactCache;
    /**
     * Semantic fuzzy matching using cosine similarity
     */
    private fuzzyMatch;
    /**
     * Transform cached output for similar but not identical code
     */
    private transformOutput;
    /**
     * GNN-based pattern matching using differentiable search
     */
    private gnnMatch;
    /**
     * Check error patterns to avoid known bad edits
     */
    private checkErrorPatterns;
    /**
     * Learn from a failed edit
     */
    private learnError;
    /**
     * Call underlying agent-booster CLI
     */
    private callAgentBooster;
    /**
     * Learn a successful pattern
     */
    private learnPattern;
    /**
     * Record co-edit relationship
     */
    private lastEditedFile;
    private recordCoEdit;
    /**
     * Detect edit type from code transformation
     */
    private detectEditType;
    /**
     * Normalize code for fuzzy matching
     */
    private normalizeCode;
    /**
     * Generate embedding
     */
    private embed;
    /**
     * Cosine similarity between two vectors
     */
    private cosineSimilarity;
    /**
     * Hash-based embedding fallback
     */
    private hashEmbed;
    /**
     * Simple string hash
     */
    private hash;
    /**
     * Extension to language mapping
     */
    private extToLanguage;
    /**
     * Update running statistics
     */
    private updateStats;
    /**
     * Record edit outcome for learning
     */
    recordOutcome(patternId: string, success: boolean): Promise<void>;
    /**
     * Get current statistics with tier distribution
     */
    getStats(): BoosterStats;
    /**
     * Persist patterns and state
     */
    persist(): Promise<void>;
    /**
     * Pretrain with expanded pattern set
     */
    pretrain(): Promise<{
        patterns: number;
        timeMs: number;
    }>;
    /**
     * Force SONA learning cycle
     */
    tick(): string | null;
    /**
     * Get intelligence stats
     */
    getIntelligenceStats(): any;
    /**
     * Get likely next files to edit
     */
    getLikelyNextFiles(filePath: string, topK?: number): Array<{
        file: string;
        score: number;
    }>;
}
export declare function getEnhancedBooster(): EnhancedAgentBooster;
/**
 * Quick apply function
 */
export declare function enhancedApply(code: string, edit: string, language: string): Promise<EnhancedEditResult>;
/**
 * Benchmark enhanced vs baseline
 */
export declare function benchmark(iterations?: number): Promise<{
    baseline: {
        avgLatency: number;
        avgConfidence: number;
    };
    enhanced: {
        avgLatency: number;
        avgConfidence: number;
        cacheHitRate: number;
        fuzzyHitRate: number;
    };
    improvement: {
        latency: string;
        confidence: string;
    };
}>;
export default EnhancedAgentBooster;
//# sourceMappingURL=agent-booster-enhanced.d.ts.map