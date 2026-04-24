/**
 * SONA Service - Self-Optimizing Neural Architecture
 *
 * Production implementation based on vibecast test-ruvector-sona patterns
 * Achieves +55.3% quality improvement with optimal configuration
 *
 * Key Performance Metrics:
 * - 2211 ops/sec throughput
 * - 18.07ms total overhead (40 layers)
 * - 0.452ms per-layer cost
 * - Sub-millisecond token latency
 *
 * Based on: https://github.com/ruvnet/vibecast/tree/claude/test-ruvector-sona-01Raj3Q3P4qjff4JGVipJhvz
 */
import { EventEmitter } from 'events';
/**
 * SONA Configuration Profiles
 */
export type SONAProfile = 'real-time' | 'batch' | 'research' | 'edge' | 'balanced' | 'custom';
export interface SONAConfig {
    profile?: SONAProfile;
    hiddenDim: number;
    embeddingDim: number;
    microLoraRank: number;
    baseLoraRank: number;
    microLoraLr: number;
    baseLoraLr: number;
    ewcLambda: number;
    patternClusters: number;
    trajectoryCapacity: number;
    backgroundIntervalMs: number;
    qualityThreshold: number;
    enableSimd: boolean;
}
export interface TrajectoryMetadata {
    id: string;
    embedding: number[];
    route?: string;
    contexts: string[];
    steps: TrajectoryStep[];
    startTime: number;
    endTime?: number;
    qualityScore?: number;
}
export interface TrajectoryStep {
    activations: number[];
    attentionWeights: number[];
    reward: number;
    timestamp: number;
}
export interface PatternMatch {
    id: string;
    centroid: number[];
    clusterSize: number;
    avgQuality: number;
    patternType: string;
    similarity: number;
}
/**
 * SONA Service - Main orchestration class
 */
export declare class SONAService extends EventEmitter {
    private engine;
    private config;
    private trajectories;
    private stats;
    constructor(config?: Partial<SONAConfig>);
    /**
     * Resolve configuration from profile or custom settings
     */
    private resolveConfig;
    /**
     * Begin a new trajectory
     */
    beginTrajectory(embedding: number[], route?: string): string;
    /**
     * Add a step to an active trajectory
     */
    addTrajectoryStep(trajectoryId: string, activations: number[], attentionWeights: number[], reward: number): void;
    /**
     * Add context to a trajectory
     */
    addTrajectoryContext(trajectoryId: string, contextId: string): void;
    /**
     * End a trajectory with quality score
     */
    endTrajectory(trajectoryId: string, qualityScore: number): void;
    /**
     * Apply Micro-LoRA to input
     */
    applyMicroLora(input: number[]): number[];
    /**
     * Apply Base-LoRA to layer
     */
    applyBaseLora(layerIndex: number, input: number[]): number[];
    /**
     * Find similar patterns
     */
    findPatterns(query: number[], k?: number): PatternMatch[];
    /**
     * Calculate cosine similarity between vectors
     */
    private calculateSimilarity;
    /**
     * Force learning cycle
     */
    forceLearn(): {
        success: boolean;
        patternsLearned: number;
    };
    /**
     * Check if learning should be triggered
     * Trigger forceLearn() when 80% of trajectory capacity is full
     */
    private checkLearningTrigger;
    /**
     * Tick for scheduled learning
     */
    tick(): void;
    /**
     * Get engine statistics
     */
    getEngineStats(): string;
    /**
     * Get service statistics
     */
    getStats(): {
        config: SONAConfig;
        engineEnabled: boolean;
        trajectoryUtilization: number;
        avgTrajectoryDuration: number;
        opsPerSecond: number;
        totalTrajectories: number;
        activeTrajectories: number;
        completedTrajectories: number;
        totalLearningCycles: number;
        avgQualityScore: number;
        totalOpsProcessed: number;
    };
    /**
     * Calculate average trajectory duration
     */
    private calculateAvgTrajectoryDuration;
    /**
     * Calculate operations per second
     */
    private calculateOpsPerSecond;
    /**
     * Enable/disable engine
     */
    setEnabled(enabled: boolean): void;
    /**
     * Check if engine is enabled
     */
    isEnabled(): boolean;
    /**
     * Flush engine state
     */
    flush(): void;
    /**
     * Get trajectory metadata
     */
    getTrajectory(trajectoryId: string): TrajectoryMetadata | undefined;
    /**
     * Get all active trajectories
     */
    getActiveTrajectories(): TrajectoryMetadata[];
}
/**
 * Create singleton SONA service instances for different use cases
 */
export declare const sonaServices: {
    realtime: SONAService;
    batch: SONAService;
    research: SONAService;
    edge: SONAService;
    balanced: SONAService;
};
/**
 * Default SONA service (balanced profile)
 */
export declare const sonaService: SONAService;
/**
 * Convenience function to create custom SONA service
 */
export declare function createSONAService(config?: Partial<SONAConfig>): SONAService;
/**
 * Example usage based on vibecast sona.test.js patterns
 */
export declare function exampleUsage(): Promise<void>;
//# sourceMappingURL=sona-service.d.ts.map