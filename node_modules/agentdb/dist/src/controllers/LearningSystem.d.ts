/**
 * Learning System - Reinforcement Learning Session Management
 *
 * Manages RL training sessions with:
 * - Session lifecycle (start/end)
 * - Action prediction with confidence scores
 * - Feedback loop for policy learning
 * - Policy training with configurable parameters
 *
 * Supports 9 RL algorithms:
 * - Q-Learning
 * - SARSA
 * - Deep Q-Network (DQN)
 * - Policy Gradient
 * - Actor-Critic
 * - Proximal Policy Optimization (PPO)
 * - Decision Transformer
 * - Monte Carlo Tree Search (MCTS)
 * - Model-Based RL
 */
type Database = any;
import { EmbeddingService } from './EmbeddingService.js';
export interface LearningSession {
    id: string;
    userId: string;
    sessionType: 'q-learning' | 'sarsa' | 'dqn' | 'policy-gradient' | 'actor-critic' | 'ppo' | 'decision-transformer' | 'mcts' | 'model-based';
    config: LearningConfig;
    startTime: number;
    endTime?: number;
    status: 'active' | 'completed' | 'failed';
    metadata?: Record<string, any>;
}
export interface LearningConfig {
    learningRate: number;
    discountFactor: number;
    explorationRate?: number;
    batchSize?: number;
    targetUpdateFrequency?: number;
}
export interface ActionPrediction {
    action: string;
    confidence: number;
    qValue?: number;
    alternatives: Array<{
        action: string;
        confidence: number;
        qValue?: number;
    }>;
}
export interface ActionFeedback {
    sessionId: string;
    action: string;
    state: string;
    reward: number;
    nextState?: string;
    success: boolean;
    timestamp: number;
}
export interface TrainingResult {
    epochsCompleted: number;
    finalLoss: number;
    avgReward: number;
    convergenceRate: number;
    trainingTimeMs: number;
}
export declare class LearningSystem {
    private db;
    private embedder;
    private activeSessions;
    constructor(db: Database, embedder: EmbeddingService);
    /**
     * Initialize database schema for learning system
     */
    private initializeSchema;
    /**
     * Start a new learning session
     */
    startSession(userId: string, sessionType: LearningSession['sessionType'], config: LearningConfig): Promise<string>;
    /**
     * End a learning session and save final policy
     */
    endSession(sessionId: string): Promise<void>;
    /**
     * Predict next action with confidence scores
     */
    predict(sessionId: string, state: string): Promise<ActionPrediction>;
    /**
     * Submit feedback for learning
     */
    submitFeedback(feedback: ActionFeedback): Promise<void>;
    /**
     * Train policy with batch learning
     */
    train(sessionId: string, epochs: number, batchSize: number, learningRate: number): Promise<TrainingResult>;
    /**
     * Get session from database
     */
    private getSession;
    /**
     * Get or create state embedding
     */
    private getStateEmbedding;
    /**
     * Get latest policy for session
     */
    private getLatestPolicy;
    /**
     * Calculate action scores based on algorithm
     */
    private calculateActionScores;
    /**
     * Update policy incrementally after feedback
     */
    private updatePolicyIncremental;
    /**
     * Train batch of experiences
     */
    private trainBatch;
    /**
     * Save policy to database
     */
    private savePolicy;
    /**
     * Calculate convergence rate
     */
    private calculateConvergenceRate;
    private calculateTransformerScore;
    private calculateUCB1;
    private calculateModelScore;
    private shuffleArray;
    /**
     * Get learning performance metrics with time windows and trends
     */
    getMetrics(options: {
        sessionId?: string;
        timeWindowDays?: number;
        includeTrends?: boolean;
        groupBy?: 'task' | 'session' | 'skill';
    }): Promise<any>;
    /**
     * Transfer learning between sessions or tasks
     */
    transferLearning(options: {
        sourceSession?: string;
        targetSession?: string;
        sourceTask?: string;
        targetTask?: string;
        minSimilarity?: number;
        transferType?: 'episodes' | 'skills' | 'causal_edges' | 'all';
        maxTransfers?: number;
    }): Promise<any>;
    /**
     * Explain action recommendations with XAI (Explainable AI)
     */
    explainAction(options: {
        query: string;
        k?: number;
        explainDepth?: 'summary' | 'detailed' | 'full';
        includeConfidence?: boolean;
        includeEvidence?: boolean;
        includeCausal?: boolean;
    }): Promise<any>;
    /**
     * Record tool execution as experience for offline learning
     */
    recordExperience(options: {
        sessionId: string;
        toolName: string;
        action: string;
        stateBefore?: any;
        stateAfter?: any;
        outcome: string;
        reward: number;
        success: boolean;
        latencyMs?: number;
        metadata?: any;
    }): Promise<number>;
    /**
     * Calculate reward signal with shaping based on multiple factors
     */
    calculateReward(options: {
        episodeId?: number;
        success: boolean;
        targetAchieved?: boolean;
        efficiencyScore?: number;
        qualityScore?: number;
        timeTakenMs?: number;
        expectedTimeMs?: number;
        includeCausal?: boolean;
        rewardFunction?: 'standard' | 'sparse' | 'dense' | 'shaped';
    }): number;
    private cosineSimilarity;
}
export {};
//# sourceMappingURL=LearningSystem.d.ts.map