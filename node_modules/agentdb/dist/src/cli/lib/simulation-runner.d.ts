/**
 * Simulation execution engine
 * Runs scenarios with configuration and tracks metrics
 */
import { type SimulationConfig } from './config-validator.js';
export interface IterationResult {
    iteration: number;
    timestamp: string;
    duration: number;
    metrics: {
        latencyUs?: {
            p50: number;
            p95: number;
            p99: number;
        };
        recallAtK?: {
            k10: number;
            k50: number;
            k100: number;
        };
        qps?: number;
        memoryMB?: number;
        [key: string]: any;
    };
    success: boolean;
    error?: string;
}
export interface SimulationReport {
    scenarioId: string;
    config: SimulationConfig;
    startTime: string;
    endTime: string;
    totalDuration: number;
    iterations: IterationResult[];
    coherenceScore: number;
    varianceMetrics: {
        latencyVariance: number;
        recallVariance: number;
        qpsVariance: number;
    };
    summary: {
        avgLatencyUs: number;
        avgRecall: number;
        avgQps: number;
        avgMemoryMB: number;
        successRate: number;
    };
    optimal: boolean;
    warnings: string[];
}
export declare class SimulationRunner {
    private scenarioRegistry;
    private scenarioCache;
    private simulationBasePath;
    constructor();
    /**
     * Find the simulation base path by searching common locations
     */
    private findSimulationBasePath;
    /**
     * Build the scenario registry with lazy loaders for all known scenarios
     */
    private buildScenarioRegistry;
    /**
     * Import a scenario module with fallback to .ts extension for development
     */
    private importScenario;
    /**
     * Get list of available scenario IDs
     */
    getAvailableScenarios(): string[];
    /**
     * Run a simulation scenario with specified configuration
     */
    runScenario(scenarioId: string, config: SimulationConfig, iterations?: number): Promise<SimulationReport>;
    /**
     * Run a single iteration
     */
    private runIteration;
    /**
     * Load scenario implementation from actual scenario files
     */
    private loadScenario;
    /**
     * Create a mock scenario for testing or when actual scenario is unavailable
     */
    private createMockScenario;
    /**
     * Extract unified metrics from scenario results
     * Normalizes different scenario output formats to a common metrics structure
     */
    private extractUnifiedMetrics;
    /**
     * Normalize metrics from various formats to unified structure
     */
    private normalizeMetrics;
    /**
     * Get scenario-specific metrics adjustments based on scenario type and config
     */
    private getScenarioSpecificMetrics;
    /**
     * Get mock metrics for testing or when actual scenario fails
     * Used as fallback when scenario files cannot be loaded
     */
    private getMockMetrics;
    /**
     * Calculate coherence score across iterations
     */
    private calculateCoherence;
    /**
     * Calculate coefficient of variation
     */
    private coefficientOfVariation;
    /**
     * Calculate variance metrics
     */
    private calculateVariance;
    /**
     * Calculate variance
     */
    private variance;
    /**
     * Calculate summary statistics
     */
    private calculateSummary;
}
//# sourceMappingURL=simulation-runner.d.ts.map