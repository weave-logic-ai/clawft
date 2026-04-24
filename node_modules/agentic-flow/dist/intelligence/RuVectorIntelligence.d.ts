/**
 * RuVector Unified Intelligence Layer
 *
 * Integrates the FULL power of RuVector ecosystem:
 *
 * @ruvector/sona - Self-Learning:
 *   - Micro-LoRA: Ultra-fast rank-1/2 adaptations (~0.1ms)
 *   - Base-LoRA: Deeper pattern adaptations
 *   - EWC++: Elastic Weight Consolidation (catastrophic forgetting prevention)
 *   - ReasoningBank: Pattern storage/retrieval via findPatterns
 *   - Trajectory tracking: Learn from execution paths
 *
 * @ruvector/attention - Advanced Attention:
 *   - MultiHeadAttention: Standard transformer attention
 *   - FlashAttention: Memory-efficient O(n) attention
 *   - HyperbolicAttention: Poincaré ball geometry for hierarchies
 *   - MoEAttention: Mixture of Experts routing
 *   - GraphRoPeAttention: Graph + Rotary Position Embeddings
 *   - EdgeFeaturedAttention: Edge-aware graph attention
 *   - DualSpaceAttention: Euclidean + Hyperbolic hybrid
 *
 * @ruvector/core - Vector Database:
 *   - HNSW indexing: 150x faster than brute force
 *   - Real vector similarity search
 *   - Cosine/Euclidean/Dot product distance
 *
 * Performance:
 *   - Micro-LoRA adaptation: ~0.1ms
 *   - FlashAttention: O(n) complexity vs O(n²)
 *   - HNSW search: O(log n) vs O(n)
 *   - Background learning: Non-blocking
 */
import { type JsSonaConfig, type JsLearnedPattern } from '@ruvector/sona';
/**
 * Intelligence Layer Configuration
 */
export interface RuVectorIntelligenceConfig {
    /** Embedding dimension (default: 384 for all-MiniLM-L6-v2) */
    embeddingDim?: number;
    /** Hidden dimension for SONA (default: 256) */
    hiddenDim?: number;
    /** Enable SONA self-learning (default: true) */
    enableSona?: boolean;
    /** SONA configuration */
    sonaConfig?: Partial<JsSonaConfig>;
    /** Attention type for agent routing (default: 'moe') */
    attentionType?: 'multi_head' | 'flash' | 'hyperbolic' | 'moe' | 'graph' | 'dual';
    /** Number of attention heads (default: 8) */
    numHeads?: number;
    /** Number of MoE experts (default: 4) */
    numExperts?: number;
    /** Top-K experts to use (default: 2) */
    topK?: number;
    /** Hyperbolic curvature for hierarchical structures (default: 1.0) */
    curvature?: number;
    /** Enable HNSW vector index (default: true) */
    enableHnsw?: boolean;
    /** HNSW M parameter - connections per node (default: 16) */
    hnswM?: number;
    /** HNSW ef_construction (default: 200) */
    hnswEfConstruction?: number;
    /** Enable trajectory tracking (default: true) */
    enableTrajectories?: boolean;
    /** Quality threshold for learning (default: 0.5) */
    qualityThreshold?: number;
    /** Background learning interval in ms (default: 60000 = 1 min) */
    backgroundIntervalMs?: number;
    /** Maximum trajectories to keep in memory (default: 1000, LRU eviction) */
    maxTrajectories?: number;
    /** Trajectory TTL in ms (default: 1800000 = 30 min) */
    trajectoryTTLMs?: number;
    /** Maximum agent embeddings to cache (default: 500, LRU eviction) */
    maxAgentEmbeddings?: number;
}
/**
 * Trajectory for learning
 */
export interface Trajectory {
    id: number;
    query: string;
    embedding: number[];
    steps: TrajectoryStep[];
    route?: string;
    contexts: string[];
    startTime: number;
}
export interface TrajectoryStep {
    action: string;
    activations: number[];
    attentionWeights: number[];
    reward: number;
    timestamp: number;
}
/**
 * Agent routing result
 */
export interface AgentRoutingResult {
    agentId: string;
    confidence: number;
    attentionWeights: Float32Array;
    expertWeights?: number[];
    latencyMs: number;
    usedHnsw: boolean;
    usedSona: boolean;
}
/**
 * Learning outcome
 */
export interface LearningOutcome {
    trajectoryId: number;
    success: boolean;
    quality: number;
    patternsLearned: number;
    adaptations: {
        microLora: boolean;
        baseLora: boolean;
        ewc: boolean;
    };
}
/**
 * RuVector Unified Intelligence Layer
 *
 * Combines SONA self-learning, advanced attention mechanisms,
 * and HNSW vector search for intelligent agent orchestration.
 */
/**
 * Result wrapper for operations that can fail
 */
export interface OperationResult<T> {
    success: boolean;
    value?: T;
    error?: string;
}
export declare class RuVectorIntelligence {
    private config;
    private initialized;
    private initPromise;
    private sona;
    private multiHeadAttention;
    private flashAttention;
    private hyperbolicAttention;
    private moeAttention;
    private graphAttention;
    private dualSpaceAttention;
    private optimizer;
    private hnswIndex;
    private trajectories;
    private trajectoryAccessOrder;
    private nextTrajectoryId;
    private agentEmbeddings;
    private agentAccessOrder;
    private learningTimer;
    private cleanupTimer;
    private stats;
    constructor(config?: RuVectorIntelligenceConfig);
    /**
     * Wait for initialization to complete
     */
    waitForInit(): Promise<void>;
    /**
     * Initialize all components (async to avoid race conditions)
     */
    private initializeAsync;
    /**
     * Start cleanup timer for stale trajectories
     */
    private startCleanupTimer;
    /**
     * Clean up trajectories older than TTL
     */
    private cleanupStaleTrajectories;
    /**
     * LRU eviction for trajectories when limit exceeded
     */
    private evictOldestTrajectory;
    /**
     * LRU eviction for agent embeddings when limit exceeded
     */
    private evictOldestAgent;
    /**
     * Update LRU access order for trajectory
     */
    private touchTrajectory;
    /**
     * Update LRU access order for agent
     */
    private touchAgent;
    /**
     * Initialize HNSW index for fast vector search
     */
    private initializeHnsw;
    /**
     * Register an agent with its embedding
     *
     * @param agentId - Unique agent identifier
     * @param embedding - Agent's semantic embedding
     * @param metadata - Optional metadata
     * @returns Operation result indicating success/failure
     */
    registerAgent(agentId: string, embedding: number[] | Float32Array, metadata?: Record<string, any>): Promise<OperationResult<void>>;
    /**
     * Route a task to the best agent using full intelligence stack
     *
     * Uses:
     * - HNSW for fast candidate retrieval
     * - Attention mechanism for ranking
     * - SONA for adaptive learning
     *
     * @param taskEmbedding - Task's semantic embedding
     * @param candidates - Optional candidate agent IDs
     * @param topK - Number of results
     */
    routeTask(taskEmbedding: number[] | Float32Array, candidates?: string[], topK?: number): Promise<AgentRoutingResult[]>;
    /**
     * Fallback attention using dot product
     */
    private computeFallbackAttention;
    /**
     * Cosine similarity between two vectors
     */
    private cosineSimilarity;
    /**
     * Begin a new trajectory for learning
     *
     * @param query - The task query
     * @param embedding - Query embedding
     * @returns Operation result with trajectory ID
     */
    beginTrajectory(query: string, embedding: number[]): OperationResult<number>;
    /**
     * Add a step to trajectory
     *
     * @param trajectoryId - Trajectory ID from beginTrajectory
     * @param action - Action taken (e.g., agent selected)
     * @param reward - Reward for this step (0-1)
     * @param activations - Optional activations
     * @param attentionWeights - Optional attention weights
     */
    addTrajectoryStep(trajectoryId: number, action: string, reward: number, activations?: number[], attentionWeights?: number[]): void;
    /**
     * Set the route (agent selected) for trajectory
     */
    setTrajectoryRoute(trajectoryId: number, route: string): void;
    /**
     * Add context to trajectory
     */
    addTrajectoryContext(trajectoryId: number, contextId: string): void;
    /**
     * End trajectory and submit for learning
     *
     * @param trajectoryId - Trajectory ID
     * @param success - Whether the task succeeded
     * @param quality - Quality score (0-1)
     * @returns Learning outcome
     */
    endTrajectory(trajectoryId: number, success: boolean, quality: number): LearningOutcome;
    /**
     * Find similar learned patterns
     *
     * Uses SONA's ReasoningBank for pattern retrieval
     *
     * @param embedding - Query embedding
     * @param k - Number of patterns to return
     */
    findPatterns(embedding: number[], k?: number): JsLearnedPattern[];
    /**
     * Force a learning cycle
     */
    forceLearning(): string;
    /**
     * Start background learning timer
     */
    private startBackgroundLearning;
    /**
     * Get patterns count from SONA stats
     */
    private getPatternsCount;
    /**
     * Compute attention asynchronously
     *
     * Useful for large batches or when non-blocking is required
     */
    computeAttentionAsync(query: Float32Array, keys: Float32Array[], values: Float32Array[], type?: 'flash' | 'hyperbolic' | 'standard'): Promise<Float32Array>;
    /**
     * Compute Poincaré distance between two embeddings
     *
     * Useful for hierarchical agent structures
     */
    poincareDistance(a: Float32Array, b: Float32Array): number;
    /**
     * Project embedding to Poincaré ball
     */
    projectToPoincare(embedding: Float32Array): Float32Array;
    /**
     * Get intelligence layer statistics
     */
    getStats(): typeof this.stats & {
        sonaStats?: any;
    };
    /**
     * Enable/disable the intelligence layer
     */
    setEnabled(enabled: boolean): void;
    /**
     * Check if enabled
     */
    isEnabled(): boolean;
    /**
     * Cleanup resources
     */
    dispose(): void;
}
/**
 * Create a default intelligence layer
 */
export declare function createIntelligenceLayer(config?: RuVectorIntelligenceConfig): RuVectorIntelligence;
/**
 * Presets for common configurations
 */
export declare const IntelligencePresets: {
    /** Fast routing with MoE and minimal learning */
    fast: {
        attentionType: "moe";
        numExperts: number;
        topK: number;
        enableTrajectories: boolean;
        backgroundIntervalMs: number;
    };
    /** Balanced performance and learning */
    balanced: {
        attentionType: "moe";
        numExperts: number;
        topK: number;
        enableTrajectories: boolean;
        backgroundIntervalMs: number;
        qualityThreshold: number;
    };
    /** Maximum learning for development */
    learning: {
        attentionType: "dual";
        enableTrajectories: boolean;
        backgroundIntervalMs: number;
        qualityThreshold: number;
        sonaConfig: {
            microLoraRank: number;
            baseLoraRank: number;
            trajectoryCapacity: number;
        };
    };
    /** Hierarchical structures (Poincaré geometry) */
    hierarchical: {
        attentionType: "hyperbolic";
        curvature: number;
        enableTrajectories: boolean;
    };
    /** Graph-based reasoning */
    graph: {
        attentionType: "graph";
        enableTrajectories: boolean;
    };
};
//# sourceMappingURL=RuVectorIntelligence.d.ts.map