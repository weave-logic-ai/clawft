/**
 * Self-Organizing HNSW Analysis
 *
 * Based on: hnsw-self-organizing.md
 * Simulates autonomous graph restructuring, adaptive parameter tuning,
 * dynamic topology evolution, and self-healing mechanisms in HNSW indexes.
 *
 * Research Foundation:
 * - Autonomous graph restructuring (MPC-based control)
 * - Adaptive parameter tuning (online learning)
 * - Dynamic topology evolution
 * - Self-healing mechanisms for deletion artifacts
 */
import type { SimulationScenario } from '../../types';
export interface SelfOrganizingMetrics {
    degradationPrevention: number;
    adaptationSpeed: number;
    autonomyScore: number;
    optimalMFound: number;
    optimalEfConstructionFound: number;
    parameterStability: number;
    initialLatencyP95Ms: number;
    day30LatencyP95Ms: number;
    latencyImprovement: number;
    fragmentationRate: number;
    healingTimeMs: number;
    postHealingRecall: number;
    memoryOverhead: number;
    cpuOverheadPercent: number;
    energyEfficiency: number;
}
export interface AdaptationStrategy {
    name: 'static' | 'mpc' | 'online-learning' | 'evolutionary' | 'hybrid';
    parameters: {
        horizon?: number;
        learningRate?: number;
        mutationRate?: number;
    };
}
/**
 * Self-Organizing HNSW Scenario
 *
 * This simulation:
 * 1. Tests autonomous graph restructuring under workload shifts
 * 2. Compares static vs self-organizing HNSW performance
 * 3. Analyzes adaptive parameter tuning effectiveness
 * 4. Measures self-healing from deletion artifacts
 * 5. Evaluates long-term stability and efficiency
 */
export declare const selfOrganizingHNSWScenario: SimulationScenario;
export default selfOrganizingHNSWScenario;
//# sourceMappingURL=self-organizing-hnsw.d.ts.map