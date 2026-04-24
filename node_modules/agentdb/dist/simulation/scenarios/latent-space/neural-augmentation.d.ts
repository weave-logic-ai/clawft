/**
 * Neural Augmentation for HNSW
 *
 * Based on: hnsw-neural-augmentation.md
 * Simulates GNN-guided edge selection, learned navigation functions,
 * embedding-topology co-optimization, and attention-based layer transitions.
 *
 * Research Foundation:
 * - GNN-guided edge selection for adaptive connectivity
 * - Learned navigation functions (RL-based)
 * - Embedding-topology joint optimization
 * - Attention-based hierarchical layer routing
 */
import type { SimulationScenario } from '../../types';
export interface NeuralAugmentationMetrics {
    edgeSelectionQuality: number;
    adaptiveConnectivity: number;
    avgDegree: number;
    sparsityGain: number;
    navigationEfficiency: number;
    avgHopsReduction: number;
    rlConvergenceEpochs: number;
    policyQuality: number;
    jointOptimizationGain: number;
    embeddingQuality: number;
    topologyQuality: number;
    layerSkipRate: number;
    routingAccuracy: number;
    speedupFromRouting: number;
}
export interface NeuralStrategy {
    name: 'baseline' | 'gnn-edges' | 'rl-nav' | 'joint-opt' | 'full-neural';
    parameters: {
        gnnLayers?: number;
        hiddenDim?: number;
        rlEpisodes?: number;
        learningRate?: number;
    };
}
/**
 * Neural Augmentation Scenario
 *
 * This simulation:
 * 1. Tests GNN-based adaptive edge selection
 * 2. Compares RL navigation vs greedy search
 * 3. Analyzes joint embedding-topology optimization
 * 4. Measures attention-based layer routing benefits
 * 5. Evaluates full neural augmentation pipeline
 */
export declare const neuralAugmentationScenario: SimulationScenario;
export default neuralAugmentationScenario;
//# sourceMappingURL=neural-augmentation.d.ts.map