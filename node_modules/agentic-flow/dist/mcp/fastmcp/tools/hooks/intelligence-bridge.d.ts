/**
 * Intelligence Bridge - Connects hooks to RuVectorIntelligence layer
 *
 * This bridges the gap between hook tools and the full RuVector ecosystem:
 * - @ruvector/sona: Micro-LoRA, EWC++, ReasoningBank, Trajectories
 * - @ruvector/attention: MoE, Flash, Hyperbolic, Graph attention
 * - ruvector core: HNSW indexing (150x faster search)
 * - TensorCompress: Tiered compression based on access frequency (v2.0.1-alpha.24+)
 *
 * Persistence: SQLite-based storage for cross-platform compatibility
 */
import { RuVectorIntelligence, type AgentRoutingResult, type Trajectory, type LearningOutcome } from '../../../../intelligence/index.js';
import { type IntelligenceStore } from '../../../../intelligence/IntelligenceStore.js';
/**
 * Get the SQLite store singleton
 */
export declare function getStore(): IntelligenceStore;
/**
 * Get the recommended algorithm for a task type
 */
export declare function getAlgorithmForTask(taskType: string): {
    algorithm: string;
    reason: string;
};
/**
 * Learn from an episode using the appropriate algorithm
 */
export declare function learnFromEpisode(taskType: string, state: string, action: string, reward: number, nextState: string, done: boolean): Promise<{
    algorithm: string;
    learned: boolean;
    qValue?: number;
}>;
/**
 * Get Q-value or policy probability for an action
 */
export declare function getActionValue(taskType: string, state: string, action: string): Promise<{
    algorithm: string;
    value: number;
    confidence: number;
}>;
/**
 * Get multi-algorithm learning stats
 */
export declare function getMultiAlgorithmStats(): Promise<{
    enabled: boolean;
    algorithms: string[];
    episodesPerAlgorithm: Record<string, number>;
    avgRewardPerAlgorithm: Record<string, number>;
}>;
/**
 * Get or create the RuVectorIntelligence singleton
 */
export declare function getIntelligence(): Promise<RuVectorIntelligence>;
/**
 * Route a task using SONA + MoE Attention + HNSW
 *
 * This replaces the simple keyword-based routing with:
 * 1. HNSW for O(log n) candidate retrieval
 * 2. Micro-LoRA transformation (~0.05ms)
 * 3. MoE attention-based ranking
 */
export declare function routeTaskIntelligent(task: string, context?: {
    file?: string;
    recentFiles?: string[];
    errorContext?: string;
}): Promise<{
    agent: string;
    confidence: number;
    routingResults: AgentRoutingResult[];
    latencyMs: number;
    usedFeatures: string[];
}>;
/**
 * Begin a trajectory for learning from task execution
 *
 * Trajectories track:
 * - Task context and embeddings
 * - Agent actions and decisions
 * - Attention patterns at each step
 * - Final outcomes for reinforcement
 */
export declare function beginTaskTrajectory(task: string, agent: string): Promise<{
    trajectoryId: number;
    success: boolean;
    error?: string;
}>;
/**
 * Record a step in the trajectory
 */
export declare function recordTrajectoryStep(trajectoryId: number, action: string, reward: number, context?: {
    file?: string;
    errorFixed?: boolean;
    testPassed?: boolean;
}): Promise<void>;
/**
 * End a trajectory and get learning outcome
 */
export declare function endTaskTrajectory(trajectoryId: number, success: boolean, quality?: number): Promise<LearningOutcome | null>;
/**
 * Store a pattern by registering it as an agent-like entity
 * Now with tiered TensorCompress for memory efficiency (v2.0.1-alpha.24+)
 */
export declare function storePattern(task: string, resolution: string, reward: number): Promise<void>;
/**
 * Find similar patterns using routing
 * Tracks access for tiered compression (v2.0.1-alpha.24+)
 */
export declare function findSimilarPatterns(task: string, topK?: number): Promise<Array<{
    task: string;
    resolution: string;
    reward: number;
    similarity: number;
}>>;
/**
 * Get intelligence stats for monitoring
 * Includes tiered compression stats (v2.0.1-alpha.24+)
 * Includes multi-algorithm learning stats (v2.0.1-alpha.25+)
 */
export declare function getIntelligenceStats(): Promise<{
    initialized: boolean;
    features: string[];
    trajectoryCount: number;
    activeTrajectories: number;
    learningEnabled: boolean;
    persistedStats?: {
        trajectories: number;
        routings: number;
        patterns: number;
        operations: number;
    };
    compressionStats?: {
        tierDistribution: {
            hot: number;
            warm: number;
            cool: number;
            cold: number;
            archive: number;
        };
        totalPatterns: number;
        totalAccesses: number;
        memorySavings: string;
    };
    multiAlgorithmStats?: {
        enabled: boolean;
        algorithms: string[];
        episodesPerAlgorithm: Record<string, number>;
        avgRewardPerAlgorithm: Record<string, number>;
    };
}>;
/**
 * Force a learning cycle (useful for batch learning)
 */
export declare function forceLearningCycle(): Promise<string>;
/**
 * Compute attention-weighted similarity for advanced routing
 */
export declare function computeAttentionSimilarity(query: Float32Array, candidates: Float32Array[]): Promise<number[]>;
/**
 * Queue an episode for batch Q-learning (3-4x faster)
 * Episodes are batched and processed in parallel
 */
export declare function queueEpisode(episode: {
    state: string;
    action: string;
    reward: number;
    nextState: string;
    done: boolean;
}): Promise<void>;
/**
 * Flush queued episodes for batch processing
 * Processes in parallel with worker threads
 */
export declare function flushEpisodeBatch(): Promise<{
    processed: number;
    parallelEnabled: boolean;
}>;
/**
 * Match patterns in parallel across multiple files
 * Provides 3-4x faster pretrain
 */
export declare function matchPatternsParallel(files: Array<{
    path: string;
    content: string;
}>): Promise<Array<{
    path: string;
    patterns: string[];
    similarity: number;
}>>;
/**
 * Index memories in background (non-blocking hooks)
 */
export declare function indexMemoriesBackground(memories: Array<{
    id: string;
    text: string;
    metadata?: Record<string, any>;
}>): Promise<{
    queued: number;
    processing: boolean;
}>;
/**
 * Parallel similarity search with sharding
 */
export declare function searchParallel(query: string, topK?: number): Promise<Array<{
    id: string;
    text: string;
    similarity: number;
}>>;
/**
 * Analyze multiple files in parallel for routing
 */
export declare function analyzeFilesParallel(files: Array<{
    path: string;
    content: string;
}>): Promise<Array<{
    path: string;
    agent: string;
    confidence: number;
}>>;
/**
 * Analyze git commits in parallel for co-edit detection
 */
export declare function analyzeCommitsParallel(commits: Array<{
    hash: string;
    message: string;
    files: string[];
}>): Promise<Array<{
    hash: string;
    coEditGroups: string[][];
    patterns: string[];
}>>;
/**
 * Get parallel stats
 */
export declare function getParallelStats(): Promise<{
    parallelEnabled: boolean;
    parallelWorkers: number;
    parallelBusy: number;
    parallelQueued: number;
}>;
/**
 * Speculatively pre-embed files that are likely to be accessed
 * Call in post-edit hook for related files
 */
export declare function speculativeEmbed(files: string[]): Promise<{
    queued: number;
}>;
/**
 * Analyze AST of multiple files in parallel
 * For pre-edit and route hooks
 */
export declare function analyzeAST(files: Array<{
    path: string;
    content: string;
}>): Promise<Array<{
    path: string;
    functions: string[];
    imports: string[];
    exports: string[];
}>>;
/**
 * Analyze code complexity metrics in parallel
 * For session-end hook to track quality
 */
export declare function analyzeComplexity(files: string[]): Promise<Array<{
    path: string;
    cyclomatic: number;
    cognitive: number;
    lines: number;
}>>;
/**
 * Build dependency graph from import statements
 * For session-start hook context
 */
export declare function buildDependencyGraph(files: string[]): Promise<{
    nodes: string[];
    edges: Array<{
        from: string;
        to: string;
    }>;
}>;
/**
 * Parallel security scan (SAST)
 * For pre-command hook before commits
 */
export declare function securityScan(files: string[]): Promise<Array<{
    path: string;
    severity: string;
    message: string;
    line: number;
}>>;
/**
 * RAG retrieval with parallel chunk processing
 * For recall hook
 */
export declare function ragRetrieve(query: string, chunks: Array<{
    id: string;
    text: string;
}>, topK?: number): Promise<Array<{
    id: string;
    text: string;
    score: number;
}>>;
/**
 * Rank context by relevance
 * For suggest-context hook
 */
export declare function rankContext(query: string, contexts: Array<{
    id: string;
    content: string;
}>): Promise<Array<{
    id: string;
    relevance: number;
}>>;
/**
 * Semantic deduplication
 * For remember hook to avoid storing duplicates
 */
export declare function deduplicate(texts: string[], threshold?: number): Promise<{
    unique: string[];
    duplicateGroups: number[][];
}>;
/**
 * Parallel git blame analysis
 * For co-edit hook
 */
export declare function gitBlame(files: string[]): Promise<Array<{
    path: string;
    authors: Array<{
        name: string;
        lines: number;
    }>;
}>>;
/**
 * Code churn metrics for routing decisions
 * For route hook to prioritize high-churn files
 */
export declare function gitChurn(patterns: string[], since?: string): Promise<Array<{
    path: string;
    commits: number;
    additions: number;
    deletions: number;
}>>;
/**
 * Get attention mechanism for specific use case
 */
export declare function getAttentionForUseCase(useCase: 'pattern-matching' | 'agent-routing' | 'code-structure' | 'context-summary' | 'multi-agent'): Promise<{
    type: string;
    instance: any;
}>;
/**
 * Parallel attention compute across multiple queries
 */
export declare function parallelAttentionCompute(queries: Float32Array[], keys: Float32Array[], values: Float32Array[], type?: 'hyperbolic' | 'flash' | 'moe'): Promise<Float32Array[]>;
/**
 * Get extended worker pool stats
 */
export declare function getExtendedWorkerStats(): Promise<{
    initialized: boolean;
    operations: string[];
}>;
export type { AgentRoutingResult, Trajectory, LearningOutcome };
//# sourceMappingURL=intelligence-bridge.d.ts.map