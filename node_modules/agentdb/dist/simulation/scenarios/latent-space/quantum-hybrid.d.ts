/**
 * Quantum-Hybrid HNSW Simulation (Theoretical)
 *
 * Based on: hnsw-quantum-hybrid.md
 * Simulates theoretical quantum-classical hybrid approaches for HNSW search
 * including quantum amplitude encoding, Grover search, and quantum walks.
 *
 * Research Foundation:
 * - Quantum amplitude encoding (simulated)
 * - Grover's algorithm for neighbor selection
 * - Quantum walks on HNSW graphs
 * - Neuromorphic integration concepts
 * - Post-classical computing projections (2040-2045)
 */
import type { SimulationScenario } from '../../types';
export interface QuantumMetrics {
    theoreticalSpeedup: number;
    groverSpeedup: number;
    quantumWalkSpeedup: number;
    qubitsRequired: number;
    gateDepth: number;
    coherenceTimeMs: number;
    errorRate: number;
    classicalFraction: number;
    quantumFraction: number;
    hybridEfficiency: number;
    current2025Viability: number;
    projected2045Viability: number;
}
export interface QuantumAlgorithm {
    name: 'classical' | 'grover' | 'quantum-walk' | 'amplitude-encoding' | 'hybrid';
    parameters: {
        neighborhoodSize?: number;
        quantumBudget?: number;
        errorTolerance?: number;
    };
}
/**
 * Quantum-Hybrid HNSW Scenario
 *
 * This simulation (THEORETICAL):
 * 1. Models quantum speedups for HNSW subroutines
 * 2. Analyzes qubit requirements for real-world graphs
 * 3. Simulates Grover search for neighbor selection
 * 4. Projects quantum walk performance on HNSW
 * 5. Evaluates hybrid classical-quantum workflows
 *
 * NOTE: This is a theoretical simulation for research purposes.
 * Actual quantum implementations require quantum hardware.
 */
export declare const quantumHybridScenario: SimulationScenario;
export default quantumHybridScenario;
//# sourceMappingURL=quantum-hybrid.d.ts.map