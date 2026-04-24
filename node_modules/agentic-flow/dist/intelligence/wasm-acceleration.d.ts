/**
 * WASM Acceleration for Intelligence Layer
 *
 * Provides WASM-accelerated components for the intelligence stack:
 * - HNSW vector indexing (150x faster pattern search)
 * - Semantic matching (intelligent agent routing)
 *
 * Uses @ruvector/edge for browser/edge compatibility.
 * Falls back to pure JS when WASM unavailable.
 */
export interface PatternEntry {
    id: string;
    embedding: Float32Array;
    metadata: Record<string, any>;
    accessCount: number;
    lastAccessed: number;
}
/**
 * WASM-accelerated pattern index for fast similarity search
 */
export declare class WasmPatternIndex {
    private hnswIndex;
    private patterns;
    private indexToId;
    private dimensions;
    constructor(dimensions?: number);
    /**
     * Initialize WASM (call once at startup)
     */
    static init(): Promise<boolean>;
    /**
     * Add pattern to index
     */
    addPattern(id: string, embedding: number[] | Float32Array, metadata?: Record<string, any>): void;
    /**
     * Search for similar patterns
     */
    searchSimilar(queryEmbedding: number[] | Float32Array, k?: number, minScore?: number): Array<{
        pattern: PatternEntry;
        distance: number;
        score: number;
    }>;
    /**
     * Get pattern by ID
     */
    getPattern(id: string): PatternEntry | undefined;
    /**
     * Get all patterns (for persistence)
     */
    getAllPatterns(): PatternEntry[];
    /**
     * Get index size
     */
    size(): number;
    /**
     * Check if WASM acceleration is active
     */
    isWasmAccelerated(): boolean;
    /**
     * Get performance stats
     */
    getStats(): {
        patternCount: number;
        wasmAccelerated: boolean;
        dimensions: number;
        totalAccesses: number;
    };
}
export interface AgentProfile {
    agentId: string;
    embedding: Float32Array;
    capabilities: string[];
    successRate: number;
    avgLatency: number;
}
/**
 * WASM-accelerated agent router for intelligent task routing
 */
export declare class WasmAgentRouter {
    private semanticMatcher;
    private agents;
    private dimensions;
    constructor(dimensions?: number);
    /**
     * Initialize WASM (call once at startup)
     */
    static init(): Promise<boolean>;
    /**
     * Register agent for routing
     */
    registerAgent(profile: AgentProfile): void;
    /**
     * Update agent metrics (success rate, latency)
     */
    updateAgentMetrics(agentId: string, successRate: number, avgLatency: number): void;
    /**
     * Route task to best agent(s)
     */
    routeTask(taskEmbedding: number[] | Float32Array, options?: {
        requiredCapabilities?: string[];
        minSuccessRate?: number;
        maxLatency?: number;
        topK?: number;
    }): Array<{
        agentId: string;
        score: number;
        capabilities: string[];
        successRate: number;
        avgLatency: number;
    }>;
    /**
     * Get agent profile
     */
    getAgent(agentId: string): AgentProfile | undefined;
    /**
     * Get all registered agents
     */
    getAllAgents(): AgentProfile[];
    /**
     * Check if WASM acceleration is active
     */
    isWasmAccelerated(): boolean;
    /**
     * Get router stats
     */
    getStats(): {
        agentCount: number;
        wasmAccelerated: boolean;
        dimensions: number;
        avgSuccessRate: number;
        avgLatency: number;
    };
}
/**
 * Get or create singleton pattern index
 */
export declare function getWasmPatternIndex(dimensions?: number): WasmPatternIndex;
/**
 * Get or create singleton agent router
 */
export declare function getWasmAgentRouter(dimensions?: number): WasmAgentRouter;
/**
 * Initialize WASM acceleration for intelligence layer
 */
export declare function initWasmAcceleration(): Promise<{
    initialized: boolean;
    patternIndex: WasmPatternIndex;
    agentRouter: WasmAgentRouter;
}>;
/**
 * Get WASM acceleration status
 */
export declare function getWasmAccelerationStatus(): {
    initialized: boolean;
    patternIndex: boolean;
    agentRouter: boolean;
    speedup: string;
};
declare const _default: {
    WasmPatternIndex: typeof WasmPatternIndex;
    WasmAgentRouter: typeof WasmAgentRouter;
    getWasmPatternIndex: typeof getWasmPatternIndex;
    getWasmAgentRouter: typeof getWasmAgentRouter;
    initWasmAcceleration: typeof initWasmAcceleration;
    getWasmAccelerationStatus: typeof getWasmAccelerationStatus;
};
export default _default;
//# sourceMappingURL=wasm-acceleration.d.ts.map