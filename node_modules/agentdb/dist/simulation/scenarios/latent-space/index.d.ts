/**
 * Latent Space Exploration Simulations - Entry Point
 *
 * Comprehensive GNN latent space analysis for AgentDB v2 with RuVector backend.
 * Validates the unique positioning as the first vector database with native GNN attention.
 *
 * Scenarios based on RuVector latent space research documents:
 * - clustering-analysis: Graph community detection (latent-graph-interplay.md)
 * - traversal-optimization: Search strategy optimization (optimization-strategies.md)
 * - hypergraph-exploration: Multi-agent relationships (advanced-architectures.md)
 * - self-organizing-hnsw: Autonomous adaptation (hnsw-self-organizing.md)
 * - neural-augmentation: GNN-enhanced HNSW (hnsw-neural-augmentation.md)
 * - quantum-hybrid: Theoretical quantum approaches (hnsw-quantum-hybrid.md)
 */
import hnswExplorationScenario from './hnsw-exploration';
import attentionAnalysisScenario from './attention-analysis';
import clusteringAnalysisScenario from './clustering-analysis';
import traversalOptimizationScenario from './traversal-optimization';
import hypergraphExplorationScenario from './hypergraph-exploration';
import selfOrganizingHNSWScenario from './self-organizing-hnsw';
import neuralAugmentationScenario from './neural-augmentation';
import quantumHybridScenario from './quantum-hybrid';
export { hnswExplorationScenario, attentionAnalysisScenario, clusteringAnalysisScenario, traversalOptimizationScenario, hypergraphExplorationScenario, selfOrganizingHNSWScenario, neuralAugmentationScenario, quantumHybridScenario, };
export declare const latentSpaceScenarios: {
    'hnsw-exploration': import("../../types").SimulationScenario;
    'attention-analysis': import("../../types").SimulationScenario;
    'clustering-analysis': import("../../types").SimulationScenario;
    'traversal-optimization': import("../../types").SimulationScenario;
    'hypergraph-exploration': import("../../types").SimulationScenario;
    'self-organizing-hnsw': import("../../types").SimulationScenario;
    'neural-augmentation': import("../../types").SimulationScenario;
    'quantum-hybrid': import("../../types").SimulationScenario;
};
export default latentSpaceScenarios;
//# sourceMappingURL=index.d.ts.map