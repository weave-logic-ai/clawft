/**
 * Graph Clustering and Community Detection Analysis
 *
 * Based on: latent-graph-interplay.md
 * Validates community detection algorithms and semantic clustering in RuVector's
 * latent space, analyzing how graph topology reflects semantic relationships.
 *
 * Research Foundation:
 * - Louvain algorithm for hierarchical community detection
 * - Label Propagation for fast clustering
 * - Graph modularity metrics
 * - Agent collaboration pattern analysis
 */
import type { SimulationScenario } from '../../types';
export interface ClusteringMetrics {
    numCommunities: number;
    communityDistribution: {
        size: number;
        count: number;
    }[];
    modularityScore: number;
    hierarchyDepth: number;
    dendrogramBalance: number;
    mergingPattern: {
        level: number;
        numMerges: number;
    }[];
    semanticPurity: number;
    crossModalAlignment: number;
    embeddingClusterOverlap: number;
    collaborationClusters: number;
    taskSpecialization: number;
    communicationEfficiency: number;
}
export interface CommunityAlgorithm {
    name: 'louvain' | 'label-propagation' | 'leiden' | 'spectral' | 'hierarchical';
    parameters: {
        resolution?: number;
        maxIterations?: number;
        threshold?: number;
    };
}
/**
 * Clustering Analysis Scenario
 *
 * This simulation:
 * 1. Runs multiple community detection algorithms
 * 2. Analyzes hierarchical structure discovery
 * 3. Validates semantic clustering quality
 * 4. Measures agent collaboration patterns
 * 5. Compares graph topology vs latent space clusters
 */
export declare const clusteringAnalysisScenario: SimulationScenario;
export default clusteringAnalysisScenario;
//# sourceMappingURL=clustering-analysis.d.ts.map