/**
 * TypeScript interfaces for SONA engine
 *
 * Replaces 'any' types with proper interfaces for type safety
 */
export interface SONAPattern {
    id: string;
    avgQuality: number;
    similarity: number;
    context?: string;
    embedding?: number[];
    route?: string;
}
export interface SONAStats {
    totalPatterns: number;
    avgQuality: number;
    trajectories: number;
    trajectoryUtilization: number;
    routes: Record<string, number>;
    qualityDistribution: {
        high: number;
        medium: number;
        low: number;
    };
}
export interface LearnResult {
    patternsLearned: number;
    quality: number;
    convergence: boolean;
    epochs: number;
}
export interface TrajectoryContext {
    route?: string;
    metadata: Record<string, string>;
}
/**
 * SONA Engine Interface
 *
 * Core methods for trajectory management and LoRA adaptation
 */
export interface SONAEngine {
    /**
     * Begin a new trajectory for learning
     * @param embedding - Initial embedding vector (should be 3072D)
     * @returns Trajectory ID
     */
    beginTrajectory(embedding: number[]): string;
    /**
     * Set the route (agent/task name) for a trajectory
     * @param id - Trajectory ID
     * @param route - Route name
     */
    setTrajectoryRoute(id: string, route: string): void;
    /**
     * Add context metadata to a trajectory
     * @param id - Trajectory ID
     * @param context - Context string (format: "key:value")
     */
    addTrajectoryContext(id: string, context: string): void;
    /**
     * Add a step to the trajectory with hidden states and attention
     * @param id - Trajectory ID
     * @param hiddenStates - Hidden layer activations
     * @param attentionWeights - Attention scores
     * @param quality - Quality score (0-1)
     */
    addTrajectoryStep(id: string, hiddenStates: number[], attentionWeights: number[], quality: number): void;
    /**
     * End a trajectory and trigger LoRA update
     * @param id - Trajectory ID
     * @param finalQuality - Final quality score (0-1)
     */
    endTrajectory(id: string, finalQuality: number): void;
    /**
     * Find similar patterns using learned LoRA weights
     * @param queryEmbedding - Query vector
     * @param k - Number of results
     * @returns Array of patterns
     */
    findPatterns(queryEmbedding: number[], k: number): SONAPattern[];
    /**
     * Apply Micro-LoRA adaptation to an embedding
     * @param embedding - Input embedding
     * @returns Adapted embedding
     */
    applyMicroLora(embedding: number[]): number[];
    /**
     * Force a learning cycle to update LoRA weights
     * @returns Learning result
     */
    forceLearn(): LearnResult;
    /**
     * Get engine statistics
     * @returns Statistics object
     */
    getStats(): SONAStats;
}
/**
 * Validation utilities
 */
export declare class ValidationUtils {
    /**
     * Validate embedding dimensions
     */
    static validateEmbedding(embedding: number[], expectedDim?: number, name?: string): void;
    /**
     * Validate quality score
     */
    static validateQuality(quality: number, name?: string): void;
    /**
     * Validate hidden states and attention weights
     */
    static validateStates(hiddenStates: number[], attention: number[], expectedDim?: number): void;
    /**
     * Sanitize file path to prevent traversal attacks
     */
    static sanitizePath(inputPath: string, baseDir?: string): string;
}
//# sourceMappingURL=sona-types.d.ts.map