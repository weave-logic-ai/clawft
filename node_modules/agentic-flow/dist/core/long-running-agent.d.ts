/**
 * Long-Running Agent with Provider Fallback
 *
 * Demonstrates how to use ProviderManager for resilient, cost-optimized agents
 * that can run for hours or days with automatic provider switching.
 */
import { ProviderConfig, FallbackStrategy } from './provider-manager.js';
export interface LongRunningAgentConfig {
    agentName: string;
    providers: ProviderConfig[];
    fallbackStrategy?: FallbackStrategy;
    checkpointInterval?: number;
    maxRuntime?: number;
    costBudget?: number;
    performanceTarget?: number;
}
export interface AgentCheckpoint {
    timestamp: Date;
    taskProgress: number;
    currentProvider: string;
    totalCost: number;
    totalTokens: number;
    completedTasks: number;
    failedTasks: number;
    state: Record<string, any>;
}
export declare class LongRunningAgent {
    private providerManager;
    private config;
    private startTime;
    private checkpoints;
    private currentState;
    private isRunning;
    private checkpointInterval?;
    constructor(config: LongRunningAgentConfig);
    /**
     * Start the agent with automatic checkpointing
     */
    start(): Promise<void>;
    /**
     * Execute a task with automatic provider fallback
     */
    executeTask<T>(task: {
        name: string;
        complexity: 'simple' | 'medium' | 'complex';
        estimatedTokens?: number;
        execute: (provider: string) => Promise<T>;
    }): Promise<T>;
    /**
     * Save checkpoint of current state
     */
    private saveCheckpoint;
    /**
     * Calculate task progress (override in subclass)
     */
    protected calculateProgress(): number;
    /**
     * Get current status
     */
    getStatus(): {
        isRunning: boolean;
        runtime: number;
        completedTasks: number;
        failedTasks: number;
        totalCost: number;
        totalTokens: number;
        providers: any[];
        lastCheckpoint?: AgentCheckpoint;
    };
    /**
     * Get detailed metrics
     */
    getMetrics(): {
        providers: import("./provider-manager.js").ProviderMetrics[];
        health: import("./provider-manager.js").ProviderHealth[];
        costs: {
            total: number;
            byProvider: Record<import("./provider-manager.js").ProviderType, number>;
            totalTokens: number;
        };
        checkpoints: AgentCheckpoint[];
    };
    /**
     * Restore from checkpoint
     */
    restoreFromCheckpoint(checkpoint: AgentCheckpoint): void;
    /**
     * Stop the agent
     */
    stop(): Promise<void>;
}
//# sourceMappingURL=long-running-agent.d.ts.map