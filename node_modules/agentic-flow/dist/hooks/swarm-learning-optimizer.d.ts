/**
 * Swarm Learning Optimizer
 *
 * Enhances parallel execution with adaptive learning, pattern recognition,
 * and automated topology selection using ReasoningBank intelligence.
 */
import { ReasoningBank } from '../reasoningbank';
export interface SwarmMetrics {
    topology: 'mesh' | 'hierarchical' | 'ring' | 'star';
    agentCount: number;
    batchSize: number;
    totalTimeMs: number;
    successRate: number;
    speedup?: number;
    taskComplexity: 'low' | 'medium' | 'high' | 'critical';
    operations: number;
}
export interface LearningPattern {
    taskType: string;
    optimalTopology: string;
    optimalBatchSize: number;
    expectedSpeedup: number;
    successRate: number;
    timestamp: string;
}
export interface OptimizationRecommendation {
    recommendedTopology: 'mesh' | 'hierarchical' | 'ring' | 'star';
    recommendedBatchSize: number;
    recommendedAgentCount: number;
    expectedSpeedup: number;
    confidence: number;
    reasoning: string;
    alternatives: Array<{
        topology: string;
        confidence: number;
        reasoning: string;
    }>;
}
export declare class SwarmLearningOptimizer {
    private reasoningBank;
    private readonly NAMESPACE;
    constructor(reasoningBank: ReasoningBank);
    /**
     * Store swarm execution metrics for learning
     */
    storeExecutionPattern(taskDescription: string, metrics: SwarmMetrics, success: boolean): Promise<void>;
    /**
     * Calculate reward score for swarm execution (0-1)
     */
    private calculateReward;
    /**
     * Generate critique for learning
     */
    private generateCritique;
    /**
     * Get optimization recommendations based on learned patterns
     */
    getOptimization(taskDescription: string, taskComplexity: 'low' | 'medium' | 'high' | 'critical', estimatedAgentCount: number): Promise<OptimizationRecommendation>;
    /**
     * Get default recommendations when no learned patterns exist
     */
    private getDefaultRecommendation;
    /**
     * Determine optimal batch size based on complexity and learned patterns
     */
    private determineOptimalBatchSize;
    /**
     * Get maximum agents for topology (to avoid coordination overhead)
     */
    private getMaxAgentsForTopology;
    /**
     * Generate statistics report on learned patterns
     */
    getOptimizationStats(): Promise<{
        totalPatterns: number;
        topologiesUsed: Record<string, number>;
        avgSpeedupByTopology: Record<string, number>;
        avgSuccessRate: number;
        bestPerformingTopology: string;
    }>;
}
/**
 * Auto-select optimal swarm configuration
 */
export declare function autoSelectSwarmConfig(reasoningBank: ReasoningBank, taskDescription: string, options?: {
    taskComplexity?: 'low' | 'medium' | 'high' | 'critical';
    estimatedAgentCount?: number;
}): Promise<OptimizationRecommendation>;
//# sourceMappingURL=swarm-learning-optimizer.d.ts.map