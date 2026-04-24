/**
 * RuvLLM Orchestrator - Self-Learning Multi-Agent Orchestration
 *
 * Integrates:
 * - TRM (Tiny Recursive Models) for multi-step reasoning
 * - SONA (Self-Optimizing Neural Architecture) for adaptive learning
 * - FastGRNN routing for intelligent agent selection
 * - ReasoningBank for pattern storage and retrieval
 *
 * Performance:
 * - 2-4x faster inference than standard transformers
 * - <100ms latency for agent routing decisions
 * - Adaptive learning from agent execution outcomes
 */
import type { ReasoningBank } from 'agentdb';
import type { EmbeddingService } from 'agentdb';
export interface TRMConfig {
    maxDepth?: number;
    beamWidth?: number;
    temperature?: number;
    minConfidence?: number;
}
export interface SONAConfig {
    learningRate?: number;
    adaptationThreshold?: number;
    enableAutoTuning?: boolean;
}
export interface AgentSelectionResult {
    agentType: string;
    confidence: number;
    reasoning: string;
    alternatives?: Array<{
        agentType: string;
        confidence: number;
    }>;
    metrics: {
        inferenceTimeMs: number;
        patternMatchScore: number;
    };
}
export interface TaskDecomposition {
    steps: Array<{
        description: string;
        estimatedComplexity: number;
        suggestedAgent: string;
    }>;
    totalComplexity: number;
    parallelizable: boolean;
}
export interface LearningOutcome {
    taskType: string;
    selectedAgent: string;
    success: boolean;
    reward: number;
    latencyMs: number;
    adaptedParameters?: Record<string, number>;
}
/**
 * RuvLLM Orchestrator
 *
 * Provides self-learning orchestration capabilities:
 * 1. Multi-step reasoning with TRM
 * 2. Adaptive agent selection with SONA
 * 3. Pattern-based learning with ReasoningBank
 * 4. Fast routing with neural architecture search
 */
export declare class RuvLLMOrchestrator {
    private reasoningBank;
    private embedder;
    private trmConfig;
    private sonaConfig;
    private agentPerformance;
    private adaptiveWeights;
    private reasoningCache;
    constructor(reasoningBank: ReasoningBank, embedder: EmbeddingService, trmConfig?: TRMConfig, sonaConfig?: SONAConfig);
    /**
     * Select the best agent for a task using TRM + SONA
     *
     * Process:
     * 1. Embed task description
     * 2. Search ReasoningBank for similar patterns
     * 3. Apply SONA adaptive weighting
     * 4. Use FastGRNN for final routing decision
     *
     * @param taskDescription - Natural language task description
     * @param context - Optional context information
     * @returns Agent selection with confidence and reasoning
     */
    selectAgent(taskDescription: string, context?: Record<string, any>): Promise<AgentSelectionResult>;
    /**
     * Decompose complex task into steps using TRM
     *
     * Recursive reasoning:
     * 1. Analyze task complexity
     * 2. Identify sub-tasks
     * 3. Assign agents to sub-tasks
     * 4. Determine execution order (sequential/parallel)
     *
     * @param taskDescription - Task to decompose
     * @param maxDepth - Maximum recursion depth
     * @returns Task decomposition with steps and agent assignments
     */
    decomposeTask(taskDescription: string, maxDepth?: number): Promise<TaskDecomposition>;
    /**
     * Record learning outcome and adapt SONA parameters
     *
     * SONA adaptation:
     * 1. Update agent performance metrics
     * 2. Adjust adaptive weights based on success/failure
     * 3. Store pattern in ReasoningBank for future retrieval
     * 4. Trigger auto-tuning if performance drops
     *
     * @param outcome - Learning outcome from agent execution
     */
    recordOutcome(outcome: LearningOutcome): Promise<void>;
    /**
     * Train GNN on accumulated patterns
     *
     * Triggers ReasoningBank GNN training with collected outcomes.
     * Should be called periodically (e.g., after N executions).
     *
     * @param options - Training options
     * @returns Training results
     */
    trainGNN(options?: {
        epochs?: number;
        batchSize?: number;
    }): Promise<{
        epochs: number;
        finalLoss: number;
    }>;
    /**
     * Get orchestrator statistics
     *
     * @returns Performance metrics and agent statistics
     */
    getStats(): {
        totalExecutions: number;
        agentPerformance: Array<{
            agent: string;
            successRate: number;
            avgLatency: number;
            uses: number;
        }>;
        cachedDecompositions: number;
    };
    /**
     * Apply SONA adaptive weighting to patterns
     */
    private applySONAWeighting;
    /**
     * Route task using FastGRNN (fast recurrent neural network)
     */
    private routeWithFastGRNN;
    /**
     * Extract agent type from reasoning pattern
     */
    private extractAgentFromPattern;
    /**
     * Infer agent from task type
     */
    private inferAgentFromTaskType;
    /**
     * Fallback agent selection when no patterns found
     */
    private fallbackAgentSelection;
    /**
     * Estimate task complexity (1-10 scale)
     */
    private estimateComplexity;
    /**
     * Identify sub-tasks for decomposition
     */
    private identifySubTasks;
    /**
     * Determine if steps can run in parallel
     */
    private canRunInParallel;
    /**
     * Adapt SONA weights based on outcome
     */
    private adaptSONAWeights;
}
//# sourceMappingURL=RuvLLMOrchestrator.d.ts.map