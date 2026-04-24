/**
 * Shared TypeScript types for AgentDB simulation framework
 */
export interface SimulationScenario {
    id: string;
    name: string;
    category: string;
    description: string;
    config: any;
    run(config: any): Promise<SimulationReport>;
}
export interface SimulationReport {
    scenarioId: string;
    timestamp: string;
    executionTimeMs: number;
    summary: Record<string, any>;
    metrics: Record<string, any>;
    detailedResults?: any[];
    analysis?: string;
    recommendations?: string[];
    artifacts?: Record<string, any>;
}
export interface UnifiedMetrics {
    latencyUs: {
        p50: number;
        p95: number;
        p99: number;
    };
    recallAtK: {
        k10: number;
        k50: number;
        k100: number;
    };
    qps: number;
    memoryMB: number;
    coherenceScore?: number;
}
export interface PerformanceMetrics {
    throughput?: number;
    latencyMs?: number;
    memoryMB?: number;
    cpuPercent?: number;
    [key: string]: any;
}
export interface SearchResult {
    id: string;
    distance: number;
    metadata?: any;
}
export interface TrainingExample {
    input: any;
    output: any;
    metadata?: any;
}
export interface SearchOptions {
    k?: number;
    ef?: number;
    filter?: any;
    [key: string]: any;
}
export interface GraphPath {
    nodes: string[];
    edges: Array<{
        from: string;
        to: string;
        weight?: number;
    }>;
    length: number;
    cost?: number;
}
export interface LouvainConfig {
    resolutionParameter: number;
    convergenceThreshold: number;
    minModularity: number;
    targetSemanticPurity: number;
    hierarchicalLevels: number;
}
export interface Community {
    id: string;
    nodes: any[];
    internalEdges: number;
    totalDegree: number;
    modularity: number;
    semanticPurity: number;
}
export interface MPCConfig {
    predictionHorizon: number;
    controlHorizon: number;
    adaptationIntervalMs: number;
    degradationThreshold: number;
    preventionRate: number;
}
export interface DegradationForecast {
    step: number;
    predictedLatency: number;
    confidence: number;
}
export interface GNNEdgeSelectionConfig {
    adaptiveM: {
        min: number;
        max: number;
    };
    targetMemoryReduction: number;
    gnnLayers: number;
    hiddenDim: number;
}
export interface RLNavigationConfig {
    trainingEpisodes: number;
    convergenceEpisodes: number;
    targetHopReduction: number;
    learningRate: number;
}
export interface JointOptimizationConfig {
    refinementCycles: number;
    targetGain: number;
    learningRate: number;
}
export interface NeuralPolicyQuality {
    quality: number;
    convergedAt: number;
    hopReduction: number;
}
export interface HypergraphConfig {
    avgHyperedgeSize: number;
    compressionRatio: number;
    cypherQueryTargetMs: number;
    taskCoverage: number;
}
export interface HyperedgeMetrics {
    size: number;
    pattern: string;
    nodes: number[];
    weight: number;
}
export interface QuantumViabilityTimeline {
    current2025: {
        viability: number;
        bottleneck: string;
    };
    nearTerm2030: {
        viability: number;
        bottleneck: string;
    };
    longTerm2040: {
        viability: number;
        ready: boolean;
    };
}
export interface QuantumHardwareProfile {
    year: number;
    qubits: number;
    errorRate: number;
    coherenceMs: number;
    viabilityScore?: number;
}
export interface TheoreticalSpeedup {
    groverSpeedup: number;
    quantumWalkSpeedup: number;
    amplitudeEncodingSpeedup: number;
    overall: number;
}
//# sourceMappingURL=types.d.ts.map