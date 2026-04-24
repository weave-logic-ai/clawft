/**
 * LearningBackend Interface - GNN self-learning capabilities (Optional)
 *
 * Provides Graph Neural Network (GNN) based learning for query enhancement
 * and adaptive pattern recognition. Available when @ruvector/gnn is installed.
 *
 * Features:
 * - Query enhancement using attention mechanisms
 * - Automatic learning from search patterns
 * - Model persistence and versioning
 */
/**
 * Type guard to check if an object implements LearningBackend
 */
export function isLearningBackend(obj) {
    return (typeof obj === 'object' &&
        obj !== null &&
        typeof obj.enhance === 'function' &&
        typeof obj.addSample === 'function' &&
        typeof obj.train === 'function' &&
        typeof obj.clearSamples === 'function' &&
        typeof obj.saveModel === 'function' &&
        typeof obj.loadModel === 'function' &&
        typeof obj.getStats === 'function' &&
        typeof obj.reset === 'function');
}
//# sourceMappingURL=LearningBackend.js.map