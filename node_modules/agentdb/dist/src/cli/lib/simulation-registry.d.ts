/**
 * Simulation Registry
 *
 * Auto-discovers and manages simulation scenarios with plugin support.
 * Provides validation, version compatibility checking, and dynamic loading.
 */
export interface SimulationMetadata {
    id: string;
    name: string;
    version: string;
    category: 'core' | 'experimental' | 'plugin';
    description: string;
    author?: string;
    agentdbVersion: string;
    tags?: string[];
    estimatedDuration?: number;
    requiredMemoryMB?: number;
}
export interface ValidationResult {
    valid: boolean;
    errors?: string[];
    warnings?: string[];
}
export interface AgentDBConfig {
    profile: 'production' | 'memory' | 'latency' | 'recall' | 'custom';
    hnsw: {
        M: number;
        efConstruction: number;
        efSearch: number;
    };
    attention: {
        heads: number;
        dimension: number;
    };
    traversal: {
        beamWidth: number;
        strategy: 'greedy' | 'beam' | 'dynamic';
    };
    clustering: {
        algorithm: 'louvain' | 'leiden' | 'spectral';
        resolution: number;
    };
    neural: {
        mode: 'none' | 'gnn-only' | 'full';
        reinforcementLearning: boolean;
    };
    hypergraph: {
        enabled: boolean;
        maxEdgeSize: number;
    };
    storage: {
        reportPath: string;
        autoBackup: boolean;
    };
    monitoring: {
        enabled: boolean;
        alertThresholds: {
            memoryMB: number;
            latencyMs: number;
        };
    };
}
export interface SimulationResult {
    id?: number;
    scenario: string;
    timestamp: Date;
    config: AgentDBConfig;
    metrics: {
        recall: number;
        latency: number;
        throughput: number;
        memoryUsage: number;
        [key: string]: any;
    };
    insights: string[];
    recommendations: string[];
    iterations?: number;
    duration?: number;
}
export interface SimulationScenario {
    metadata: SimulationMetadata;
    execute(config: AgentDBConfig): Promise<SimulationResult>;
    validate?(config: AgentDBConfig): ValidationResult;
    cleanup?(): Promise<void>;
}
export declare class SimulationRegistry {
    private scenarios;
    private discoveryPaths;
    private agentdbVersion;
    constructor(agentdbVersion?: string);
    /**
     * Discover all simulation scenarios from configured paths.
     */
    discover(): Promise<SimulationScenario[]>;
    /**
     * Discover scenarios in a specific directory.
     */
    private discoverInPath;
    /**
     * Load a scenario implementation from a directory.
     */
    private loadScenario;
    /**
     * Extract metadata from package.json.
     */
    private extractMetadataFromPackage;
    /**
     * Get scenario by ID.
     */
    get(id: string): SimulationScenario | undefined;
    /**
     * List all scenarios.
     */
    list(): SimulationScenario[];
    /**
     * Filter scenarios by category.
     */
    listByCategory(category: SimulationMetadata['category']): SimulationScenario[];
    /**
     * Search scenarios by tags.
     */
    searchByTags(tags: string[]): SimulationScenario[];
    /**
     * Register a scenario manually (for testing or runtime registration).
     */
    register(scenario: SimulationScenario): void;
    /**
     * Unregister a scenario.
     */
    unregister(id: string): boolean;
    /**
     * Add a custom discovery path.
     */
    addDiscoveryPath(path: string): void;
    /**
     * Validate scenario implementation.
     */
    validate(scenario: SimulationScenario): ValidationResult;
    /**
     * Check version compatibility.
     */
    isCompatible(scenario: SimulationScenario): boolean;
    /**
     * Get scenarios grouped by category.
     */
    getGroupedByCategory(): Record<string, SimulationScenario[]>;
    /**
     * Get scenario statistics.
     */
    getStats(): {
        total: number;
        byCategory: Record<string, number>;
        compatible: number;
        incompatible: number;
    };
    /**
     * Generate registry report.
     */
    generateReport(): string;
}
/**
 * Create and initialize a registry with auto-discovery.
 */
export declare function createRegistry(agentdbVersion?: string): Promise<SimulationRegistry>;
//# sourceMappingURL=simulation-registry.d.ts.map