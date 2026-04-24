/**
 * TinyDancer Router - FastGRNN-based Neural Agent Routing
 *
 * Integrates @ruvector/tiny-dancer for production-grade AI agent orchestration.
 *
 * Features:
 * - FastGRNN Neural Routing: Efficient gated recurrent network for fast inference
 * - Uncertainty Estimation: Know when the router is confident vs. uncertain
 * - Circuit Breaker: Automatic fallback when routing fails repeatedly
 * - Hot-Reload: Update models without restarting the application
 * - SIMD Optimized: Native Rust performance with SIMD acceleration
 *
 * Performance:
 * - <5ms routing decisions
 * - 99.9% uptime with circuit breaker
 * - Adaptive learning from routing outcomes
 */
export interface RouteResult {
    agentId: string;
    confidence: number;
    uncertainty?: number;
    alternatives?: Array<{
        agentId: string;
        confidence: number;
    }>;
    latencyMs: number;
}
export interface RouterMetrics {
    totalRoutes: number;
    avgLatencyMs: number;
    uncertaintyDistribution: {
        low: number;
        medium: number;
        high: number;
    };
    agentDistribution: Map<string, number>;
}
export interface TinyDancerConfig {
    /** Embedding dimension (default: 384 for all-MiniLM-L6-v2) */
    embeddingDim?: number;
    /** Number of agents to route to */
    numAgents?: number;
    /** Temperature for softmax (lower = more confident) */
    temperature?: number;
    /** Enable uncertainty estimation */
    enableUncertainty?: boolean;
    /** Circuit breaker failure threshold */
    circuitBreakerThreshold?: number;
    /** Fallback agent when circuit opens */
    fallbackAgent?: string;
}
/**
 * Initialize TinyDancer module
 */
export declare function initTinyDancer(): Promise<boolean>;
/**
 * Check if TinyDancer is available
 */
export declare function isTinyDancerAvailable(): boolean;
/**
 * TinyDancer Router - Neural agent routing with circuit breaker
 */
export declare class TinyDancerRouter {
    private config;
    private nativeRouter;
    private circuitBreaker;
    private agentWeights;
    private routingHistory;
    private totalRoutes;
    private totalLatencyMs;
    private uncertaintyCounts;
    private agents;
    constructor(config?: TinyDancerConfig);
    /**
     * Initialize native TinyDancer router
     */
    private initializeNative;
    /**
     * Register an agent for routing
     */
    registerAgent(agentId: string, embedding: Float32Array | number[], capabilities: string[]): void;
    /**
     * Route task to best agent
     *
     * Uses FastGRNN neural routing if available, falls back to
     * cosine similarity matching otherwise.
     *
     * @param taskEmbedding - Embedding of the task description
     * @returns Route result with selected agent and confidence
     */
    route(taskEmbedding: Float32Array | number[]): Promise<RouteResult>;
    /**
     * Route multiple tasks in batch (parallel processing)
     */
    routeBatch(taskEmbeddings: Float32Array[]): Promise<RouteResult[]>;
    /**
     * Get uncertainty estimate for a task
     */
    getUncertainty(taskEmbedding: Float32Array): Promise<number>;
    /**
     * Record routing outcome for learning
     */
    recordOutcome(agentId: string, success: boolean, reward?: number): void;
    /**
     * Hot-reload model without restart
     */
    hotReload(modelPath: string): Promise<void>;
    /**
     * Get routing metrics
     */
    getMetrics(): RouterMetrics;
    /**
     * Get circuit breaker state
     */
    getCircuitState(): 'closed' | 'open' | 'half-open' | 'unknown';
    /**
     * Reset circuit breaker
     */
    resetCircuit(): void;
    /**
     * Check if using native TinyDancer
     */
    isNative(): boolean;
    /**
     * Cleanup resources
     */
    shutdown(): Promise<void>;
    /**
     * Fallback routing using cosine similarity
     */
    private fallbackRoute;
    /**
     * Compute similarities to all agents
     */
    private computeSimilarities;
    /**
     * Estimate uncertainty from score distribution
     */
    private estimateUncertaintyFromScores;
    /**
     * Cosine similarity between two vectors
     */
    private cosineSimilarity;
}
/**
 * Get global TinyDancer router instance
 */
export declare function getTinyDancerRouter(config?: TinyDancerConfig): TinyDancerRouter;
/**
 * Reset global router (for testing)
 */
export declare function resetTinyDancerRouter(): Promise<void>;
declare const _default: {
    TinyDancerRouter: typeof TinyDancerRouter;
    getTinyDancerRouter: typeof getTinyDancerRouter;
    resetTinyDancerRouter: typeof resetTinyDancerRouter;
    initTinyDancer: typeof initTinyDancer;
    isTinyDancerAvailable: typeof isTinyDancerAvailable;
};
export default _default;
//# sourceMappingURL=TinyDancerRouter.d.ts.map