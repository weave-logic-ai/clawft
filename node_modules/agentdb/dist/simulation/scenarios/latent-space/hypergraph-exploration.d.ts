/**
 * Hypergraph Exploration for Multi-Agent Relationships
 *
 * Based on: advanced-architectures.md
 * Explores hypergraph structures (3+ node relationships) for modeling
 * complex multi-agent collaboration patterns and causal relationships.
 *
 * Research Foundation:
 * - Hyperedges connecting 3+ nodes
 * - Multi-agent collaboration semantics
 * - Complex causal relationship modeling
 * - Cypher query performance on hypergraphs
 */
import type { SimulationScenario } from '../../types';
export interface HypergraphMetrics {
    numNodes: number;
    numHyperedges: number;
    avgHyperedgeSize: number;
    maxHyperedgeSize: number;
    hypergraphDensity: number;
    clusteringCoefficient: number;
    smallWorldness: number;
    collaborationGroups: number;
    avgGroupSize: number;
    taskCoverage: number;
    cypherQueryLatencyMs: number;
    hyperedgeTraversalMs: number;
    patternMatchingMs: number;
    causalChainLength: number;
    causalBranchingFactor: number;
    transitivityScore: number;
}
export interface HyperedgeType {
    type: 'collaboration' | 'causal' | 'dependency' | 'composition';
    nodes: number[];
    weight: number;
    metadata?: any;
}
/**
 * Hypergraph Exploration Scenario
 *
 * This simulation:
 * 1. Constructs hypergraphs with 3+ node relationships
 * 2. Models multi-agent collaboration patterns
 * 3. Analyzes complex causal relationships
 * 4. Benchmarks Cypher query performance
 * 5. Compares hypergraph vs standard graph representations
 */
export declare const hypergraphExplorationScenario: SimulationScenario;
export default hypergraphExplorationScenario;
//# sourceMappingURL=hypergraph-exploration.d.ts.map