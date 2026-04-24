/**
 * E2B Swarm Optimizer - Performance optimization for E2B swarms
 *
 * Provides automatic optimization strategies:
 * - Agent pool sizing
 * - Task batching
 * - Load rebalancing
 * - Resource cleanup
 */
import { E2BSwarmOrchestrator, E2BSwarmMetrics } from "./e2b-swarm.js";
/**
 * Optimization configuration
 */
export interface OptimizationConfig {
    targetUtilization: number;
    maxErrorRate: number;
    minAgents: number;
    maxAgents: number;
    scaleUpThreshold: number;
    scaleDownThreshold: number;
    batchSize: number;
    optimizationInterval: number;
}
/**
 * Optimization report
 */
export interface OptimizationReport {
    timestamp: number;
    metrics: E2BSwarmMetrics;
    recommendations: OptimizationRecommendation[];
    actionsApplied: string[];
    healthScore: number;
}
/**
 * Optimization recommendation
 */
export interface OptimizationRecommendation {
    type: 'scale-up' | 'scale-down' | 'rebalance' | 'cleanup' | 'batch' | 'capability-add';
    priority: 'low' | 'medium' | 'high' | 'critical';
    description: string;
    impact: string;
    autoApply: boolean;
}
/**
 * E2B Swarm Optimizer
 */
export declare class E2BSwarmOptimizer {
    private swarm;
    private config;
    private optimizationHistory;
    private intervalId;
    constructor(swarm: E2BSwarmOrchestrator, config?: Partial<OptimizationConfig>);
    /**
     * Start automatic optimization
     */
    startAutoOptimization(): void;
    /**
     * Stop automatic optimization
     */
    stopAutoOptimization(): void;
    /**
     * Run optimization cycle
     */
    optimize(): Promise<OptimizationReport>;
    /**
     * Analyze metrics and generate recommendations
     */
    private analyzeMetrics;
    /**
     * Apply an optimization recommendation
     */
    private applyRecommendation;
    /**
     * Cleanup failing agents
     */
    private cleanupFailingAgents;
    /**
     * Rebalance load across agents
     */
    private rebalanceLoad;
    /**
     * Calculate average utilization
     */
    private calculateAverageUtilization;
    /**
     * Calculate health score (0-100)
     */
    private calculateHealthScore;
    /**
     * Get optimization history
     */
    getHistory(): OptimizationReport[];
    /**
     * Get current health score
     */
    getHealthScore(): number;
    /**
     * Get optimization summary
     */
    getSummary(): {
        healthScore: number;
        totalOptimizations: number;
        lastOptimization: number | null;
        actionsApplied: number;
        currentMetrics: E2BSwarmMetrics;
    };
}
/**
 * Create optimizer for a swarm
 */
export declare function createSwarmOptimizer(swarm: E2BSwarmOrchestrator, config?: Partial<OptimizationConfig>): E2BSwarmOptimizer;
/**
 * Quick optimization pass
 */
export declare function optimizeSwarm(swarm: E2BSwarmOrchestrator): Promise<OptimizationReport>;
//# sourceMappingURL=e2b-swarm-optimizer.d.ts.map