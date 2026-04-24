/**
 * AttentionService - RuVector Attention Mechanisms Integration
 *
 * This service provides a unified interface for attention mechanisms with
 * RuVector WASM/NAPI bindings and robust JavaScript fallback implementations.
 *
 * Architecture:
 * - HyperbolicAttention: Tree-structured Poincare embeddings for causal chains
 * - FlashAttention: Memory-efficient block-wise attention for consolidation
 * - GraphRoPE: Hop-distance-aware positional encoding for graph queries
 * - MoEAttention: Expert routing for specialized memory domains
 *
 * All mechanisms default to FALSE (opt-in) and provide backward-compatible fallbacks.
 * WASM bindings are used when available for 10-100x performance improvements.
 *
 * @module AttentionService
 * @version 2.0.0-alpha.4
 */
type Database = any;
interface PerformanceLog {
    mechanism: string;
    backend: 'wasm' | 'napi' | 'fallback';
    durationMs: number;
    inputSize: number;
    timestamp: number;
}
/**
 * Get all performance logs (useful for debugging and optimization)
 */
export declare function getPerformanceLogs(): PerformanceLog[];
/**
 * Clear performance logs
 */
export declare function clearPerformanceLogs(): void;
/**
 * Check if WASM backend is available
 */
export declare function isWasmAvailable(): boolean;
/**
 * Check if NAPI backend is available
 */
export declare function isNapiAvailable(): boolean;
/**
 * Get the best available backend
 */
export declare function getAvailableBackend(): 'napi' | 'wasm' | 'fallback';
/**
 * Configuration for HyperbolicAttention
 * Uses Poincaré ball model for hierarchical causal relationships
 */
export interface HyperbolicAttentionConfig {
    /** Enable hyperbolic attention (default: false) */
    enabled: boolean;
    /** Curvature of Poincaré ball (default: 1.0) */
    curvature?: number;
    /** Embedding dimension (default: 384) */
    dimension?: number;
    /** Temperature for attention softmax (default: 1.0) */
    temperature?: number;
}
/**
 * Configuration for FlashAttention
 * Block-wise memory-efficient attention for large buffers
 */
export interface FlashAttentionConfig {
    /** Enable flash attention (default: false) */
    enabled: boolean;
    /** Block size for tiling (default: 256) */
    blockSize?: number;
    /** Use SIMD acceleration (default: true) */
    useSIMD?: boolean;
    /** Maximum sequence length (default: 4096) */
    maxSeqLen?: number;
}
/**
 * Configuration for GraphRoPE
 * Rotary positional encoding aware of graph hop distances
 */
export interface GraphRoPEConfig {
    /** Enable graph RoPE (default: false) */
    enabled: boolean;
    /** Maximum hop distance (default: 10) */
    maxHops?: number;
    /** Rotary dimension (default: 64) */
    rotaryDim?: number;
    /** Base frequency (default: 10000) */
    baseFreq?: number;
}
/**
 * Configuration for MoEAttention
 * Mixture-of-Experts routing for specialized domains
 */
export interface MoEAttentionConfig {
    /** Enable MoE attention (default: false) */
    enabled: boolean;
    /** Number of experts (default: 8) */
    numExperts?: number;
    /** Top-k experts to route to (default: 2) */
    topK?: number;
    /** Expert specialization domains */
    expertDomains?: string[];
}
/**
 * Result from HyperbolicAttention computation
 */
export interface HyperbolicAttentionResult {
    /** Attended embeddings in Poincaré space */
    attended: Float32Array;
    /** Attention weights */
    weights: Float32Array;
    /** Hierarchical distances */
    distances: number[];
    /** Performance metrics */
    metrics: {
        computeTimeMs: number;
        memoryUsedMB: number;
    };
}
/**
 * Result from FlashAttention computation
 */
export interface FlashAttentionResult {
    /** Consolidated output */
    output: Float32Array;
    /** Attention scores (if requested) */
    scores?: Float32Array;
    /** Performance metrics */
    metrics: {
        computeTimeMs: number;
        peakMemoryMB: number;
        blocksProcessed: number;
    };
}
/**
 * Result from GraphRoPE computation
 */
export interface GraphRoPEResult {
    /** Position-encoded queries */
    queries: Float32Array;
    /** Position-encoded keys */
    keys: Float32Array;
    /** Hop-distance aware encodings */
    hopEncodings: Float32Array;
    /** Performance metrics */
    metrics: {
        computeTimeMs: number;
    };
}
/**
 * Result from MoEAttention computation
 */
export interface MoEAttentionResult {
    /** Routed output from experts */
    output: Float32Array;
    /** Expert assignments per query */
    expertAssignments: number[][];
    /** Expert weights per query */
    expertWeights: number[][];
    /** Performance metrics */
    metrics: {
        computeTimeMs: number;
        expertsUsed: number;
        routingEntropy: number;
    };
}
/**
 * AttentionService - Unified interface for attention mechanisms
 *
 * Provides fallback implementations until RuVector WASM/NAPI bindings are available.
 * All mechanisms are opt-in via configuration flags.
 */
export declare class AttentionService {
    private db;
    private hyperbolicConfig;
    private flashConfig;
    private graphRoPEConfig;
    private moeConfig;
    constructor(db: Database, configs?: {
        hyperbolic?: Partial<HyperbolicAttentionConfig>;
        flash?: Partial<FlashAttentionConfig>;
        graphRoPE?: Partial<GraphRoPEConfig>;
        moe?: Partial<MoEAttentionConfig>;
    });
    /**
     * HyperbolicAttention: Tree-structured Poincare attention for causal chains
     *
     * Uses hyperbolic geometry to model hierarchical relationships in causal memory.
     * Attempts to use NAPI bindings first, then WASM, then falls back to JavaScript.
     *
     * @param queries - Query embeddings [num_queries, dim]
     * @param keys - Key embeddings from causal chain [num_keys, dim]
     * @param values - Value embeddings [num_keys, dim]
     * @param hierarchyLevels - Hierarchy level for each key (0 = root)
     * @returns Attention result with Poincare-weighted outputs
     */
    hyperbolicAttention(queries: Float32Array, keys: Float32Array, values: Float32Array, hierarchyLevels: number[]): Promise<HyperbolicAttentionResult>;
    /**
     * FlashAttention: Memory-efficient block-wise attention for consolidation
     *
     * Processes attention in blocks to reduce peak memory usage.
     * Ideal for episodic memory consolidation with large buffers.
     * Attempts to use NAPI bindings first, then WASM, then falls back to JavaScript.
     *
     * @param queries - Query embeddings [num_queries, dim]
     * @param keys - Key embeddings [num_keys, dim]
     * @param values - Value embeddings [num_keys, dim]
     * @returns Attention result with memory-efficient computation
     */
    flashAttention(queries: Float32Array, keys: Float32Array, values: Float32Array): Promise<FlashAttentionResult>;
    /**
     * GraphRoPE: Hop-distance-aware rotary positional encoding
     *
     * Encodes graph distances into query/key representations using rotary
     * positional embeddings. Attempts to use NAPI bindings first, then falls
     * back to JavaScript implementation.
     *
     * Note: The WASM module does not have a direct GraphRoPE binding, so we
     * use the NAPI GraphRoPeAttention class when available.
     *
     * @param queries - Query embeddings [num_queries, dim]
     * @param keys - Key embeddings [num_keys, dim]
     * @param hopDistances - Hop distance matrix [num_queries, num_keys]
     * @returns Position-encoded queries and keys
     */
    graphRoPE(queries: Float32Array, keys: Float32Array, hopDistances: number[][]): Promise<GraphRoPEResult>;
    /**
     * MoEAttention: Mixture-of-Experts routing for specialized domains
     *
     * Routes queries to specialized expert networks based on domain.
     * Ideal for ReasoningBank with diverse pattern types.
     * Attempts to use NAPI bindings first, then WASM, then falls back to JavaScript.
     *
     * @param queries - Query embeddings [num_queries, dim]
     * @param keys - Key embeddings [num_keys, dim]
     * @param values - Value embeddings [num_keys, dim]
     * @param domains - Domain labels for each key
     * @returns Expert-routed attention output
     */
    moeAttention(queries: Float32Array, keys: Float32Array, values: Float32Array, domains: string[]): Promise<MoEAttentionResult>;
    private fallbackHyperbolicAttention;
    private fallbackFlashAttention;
    private fallbackGraphRoPE;
    private fallbackMoEAttention;
    private softmax;
    private calculateEntropy;
    /**
     * Get current configuration
     */
    getConfig(): {
        hyperbolic: HyperbolicAttentionConfig;
        flash: FlashAttentionConfig;
        graphRoPE: GraphRoPEConfig;
        moe: MoEAttentionConfig;
    };
    /**
     * Update configuration dynamically
     */
    updateConfig(configs: {
        hyperbolic?: Partial<HyperbolicAttentionConfig>;
        flash?: Partial<FlashAttentionConfig>;
        graphRoPE?: Partial<GraphRoPEConfig>;
        moe?: Partial<MoEAttentionConfig>;
    }): void;
    /**
     * Get backend status information
     * Useful for debugging and monitoring
     */
    getBackendStatus(): {
        wasmAvailable: boolean;
        napiAvailable: boolean;
        activeBackend: 'napi' | 'wasm' | 'fallback';
        performanceLoggingEnabled: boolean;
    };
}
/**
 * Initialize attention backends proactively
 * Call this at application startup for faster first-use performance
 *
 * @returns Promise resolving to backend status
 */
export declare function initializeAttentionBackends(): Promise<{
    wasmLoaded: boolean;
    napiLoaded: boolean;
    recommendedBackend: 'napi' | 'wasm' | 'fallback';
}>;
/**
 * Create an AttentionService with all mechanisms enabled
 * Convenience factory for quick setup
 *
 * @param db - Database instance (can be null for standalone usage)
 * @returns Configured AttentionService instance
 */
export declare function createAttentionService(db?: Database | null): AttentionService;
/**
 * Create an AttentionService with only fallback implementations
 * Useful for environments where WASM/NAPI are unavailable
 *
 * @param db - Database instance (can be null for standalone usage)
 * @returns Configured AttentionService instance with fallbacks only
 */
export declare function createFallbackAttentionService(db?: Database | null): AttentionService;
export {};
//# sourceMappingURL=AttentionService.d.ts.map