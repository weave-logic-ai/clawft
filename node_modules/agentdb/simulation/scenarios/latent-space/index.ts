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

export {
  hnswExplorationScenario,
  attentionAnalysisScenario,
  clusteringAnalysisScenario,
  traversalOptimizationScenario,
  hypergraphExplorationScenario,
  selfOrganizingHNSWScenario,
  neuralAugmentationScenario,
  quantumHybridScenario,
};

export const latentSpaceScenarios = {
  'hnsw-exploration': hnswExplorationScenario,
  'attention-analysis': attentionAnalysisScenario,
  'clustering-analysis': clusteringAnalysisScenario,
  'traversal-optimization': traversalOptimizationScenario,
  'hypergraph-exploration': hypergraphExplorationScenario,
  'self-organizing-hnsw': selfOrganizingHNSWScenario,
  'neural-augmentation': neuralAugmentationScenario,
  'quantum-hybrid': quantumHybridScenario,
};

export default latentSpaceScenarios;
