/**
 * Federated Learning Integration for SONA v0.1.4
 *
 * Provides distributed learning capabilities with WasmEphemeralAgent and WasmFederatedCoordinator
 */
import type { SonaEngine } from '@ruvector/sona';
/**
 * Federated agent state for synchronization
 */
export interface FederatedAgentState {
    agentId: string;
    embedding: Float32Array;
    quality: number;
    timestamp: number;
    metadata?: Record<string, any>;
}
/**
 * Federated learning configuration
 */
export interface FederatedConfig {
    /** Agent identifier */
    agentId: string;
    /** Coordinator endpoint (for agents) or null (for coordinator) */
    coordinatorEndpoint?: string;
    /** Minimum quality threshold for aggregation */
    minQuality?: number;
    /** Aggregation interval in milliseconds */
    aggregationInterval?: number;
    /** Enable quality filtering */
    qualityFiltering?: boolean;
    /** Maximum agents to aggregate */
    maxAgents?: number;
}
/**
 * Ephemeral agent for lightweight distributed learning
 *
 * Features:
 * - ~5MB footprint
 * - Local task processing
 * - State export for federation
 * - Quality-based filtering
 */
export declare class EphemeralLearningAgent {
    private agentId;
    private sonaEngine;
    private taskHistory;
    private config;
    constructor(config: FederatedConfig);
    /**
     * Initialize SONA engine with ephemeral preset
     */
    initialize(sonaEngine: SonaEngine): Promise<void>;
    /**
     * Process a task and update local learning state
     */
    processTask(embedding: Float32Array, quality: number): Promise<void>;
    /**
     * Export agent state for federation
     */
    exportState(): FederatedAgentState;
    /**
     * Import consolidated state from coordinator
     */
    importState(state: FederatedAgentState): Promise<void>;
    /**
     * Clear task history (after successful aggregation)
     */
    clearHistory(): void;
    /**
     * Get current task count
     */
    getTaskCount(): number;
    /**
     * Compute average embedding from multiple embeddings
     */
    private computeAverageEmbedding;
}
/**
 * Federated coordinator for central aggregation
 *
 * Features:
 * - Aggregate states from multiple agents
 * - Quality-based filtering
 * - Consolidated model distribution
 * - Scalable to hundreds of agents
 */
export declare class FederatedLearningCoordinator {
    private coordinatorId;
    private agentStates;
    private consolidatedState;
    private config;
    constructor(config: FederatedConfig);
    /**
     * Aggregate state from an agent
     */
    aggregate(state: FederatedAgentState): Promise<void>;
    /**
     * Consolidate all agent states into unified model
     */
    consolidate(): Promise<FederatedAgentState>;
    /**
     * Get consolidated state for distribution to agents
     */
    getConsolidatedState(): FederatedAgentState | null;
    /**
     * Clear all agent states (after distribution)
     */
    clearStates(): void;
    /**
     * Get number of aggregated agents
     */
    getAgentCount(): number;
    /**
     * Get agent states summary
     */
    getSummary(): {
        agentCount: number;
        avgQuality: number;
        minQuality: number;
        maxQuality: number;
        consolidated: boolean;
    };
}
/**
 * Federated learning manager for coordinating multiple agents
 */
export declare class FederatedLearningManager {
    private coordinator;
    private agents;
    private aggregationTimer;
    constructor(config: FederatedConfig);
    /**
     * Register a new agent
     */
    registerAgent(agentId: string, sonaEngine: SonaEngine): EphemeralLearningAgent;
    /**
     * Start automatic aggregation
     */
    startAggregation(intervalMs?: number): void;
    /**
     * Stop automatic aggregation
     */
    stopAggregation(): void;
    /**
     * Aggregate all agent states
     */
    aggregateAll(): Promise<void>;
    /**
     * Get aggregation summary
     */
    getSummary(): {
        coordinator: {
            agentCount: number;
            avgQuality: number;
            minQuality: number;
            maxQuality: number;
            consolidated: boolean;
        };
        agents: {
            count: number;
            activeAgents: {
                id: string;
                taskCount: number;
            }[];
        };
    };
    /**
     * Cleanup resources
     */
    cleanup(): void;
}
//# sourceMappingURL=federated-learning.d.ts.map