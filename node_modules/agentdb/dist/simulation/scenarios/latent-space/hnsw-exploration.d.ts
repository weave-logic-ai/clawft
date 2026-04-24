/**
 * HNSW Latent Space Exploration Simulation
 *
 * Analyzes the hierarchical navigable small world graph structure created by RuVector's
 * HNSW implementation, comparing against traditional hnswlib performance and validating
 * the graph properties that enable sub-millisecond search.
 *
 * Research Foundation:
 * - RuVector HNSW: 61µs search latency (k=10, 384d)
 * - hnswlib baseline: ~500µs
 * - Target: 8x speedup with native Rust implementation
 */
import type { SimulationScenario } from '../../types';
export interface HNSWGraphMetrics {
    layers: number;
    nodesPerLayer: number[];
    connectivityDistribution: {
        layer: number;
        avgDegree: number;
        maxDegree: number;
    }[];
    averagePathLength: number;
    clusteringCoefficient: number;
    smallWorldIndex: number;
    smallWorldFormula?: {
        C: number;
        C_random: number;
        L: number;
        L_random: number;
        sigma: number;
    };
    searchPathLength: {
        percentile: number;
        hops: number;
    }[];
    layerTraversalCounts: number[];
    greedySearchSuccess: number;
    buildTimeMs: number;
    searchLatencyUs: {
        k: number;
        p50: number;
        p95: number;
        p99: number;
    }[];
    memoryUsageBytes: number;
}
export interface HNSWComparisonMetrics {
    backend: 'ruvector-gnn' | 'ruvector-core' | 'hnswlib';
    vectorCount: number;
    dimension: number;
    M: number;
    efConstruction: number;
    efSearch: number;
    graphMetrics: HNSWGraphMetrics;
    recallAtK: {
        k: number;
        recall: number;
    }[];
    qps: number;
    speedupVsBaseline: number;
}
/**
 * HNSW Graph Exploration Scenario
 *
 * This simulation:
 * 1. Builds HNSW indexes with different backends and parameters
 * 2. Analyzes graph topology and small-world properties
 * 3. Measures search efficiency and path characteristics
 * 4. Compares RuVector vs hnswlib performance
 * 5. Validates sub-millisecond latency claims
 */
export declare const hnswExplorationScenario: SimulationScenario;
export default hnswExplorationScenario;
//# sourceMappingURL=hnsw-exploration.d.ts.map