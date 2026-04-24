/**
 * Configuration Manager
 *
 * Centralized configuration management with profiles, validation,
 * and environment variable support.
 */
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
    logging?: {
        level: 'debug' | 'info' | 'warn' | 'error';
        file?: string;
    };
}
export interface ValidationResult {
    valid: boolean;
    errors?: string[];
    warnings?: string[];
}
export declare const PRESET_PROFILES: Record<string, Omit<AgentDBConfig, 'profile'>>;
export declare class ConfigManager {
    private ajv;
    private validator;
    constructor();
    /**
     * Load configuration from file.
     */
    loadFromFile(filePath: string): AgentDBConfig;
    /**
     * Load configuration from preset profile.
     */
    loadProfile(profile: keyof typeof PRESET_PROFILES): AgentDBConfig;
    /**
     * Load configuration with environment variable overrides.
     */
    loadWithEnv(baseConfig: AgentDBConfig): AgentDBConfig;
    /**
     * Load configuration from default locations.
     * Priority: CLI args > .agentdb.json > ~/.agentdb/config.json > defaults
     */
    loadDefault(profile?: string): AgentDBConfig;
    /**
     * Save configuration to file.
     */
    save(config: AgentDBConfig, filePath: string): void;
    /**
     * Validate configuration against schema.
     */
    validate(config: any): AgentDBConfig;
    /**
     * Validate with warnings (non-throwing).
     */
    validateWithWarnings(config: any): ValidationResult;
    /**
     * Merge configurations (deep merge).
     */
    merge(base: AgentDBConfig, override: Partial<AgentDBConfig>): AgentDBConfig;
    /**
     * Get configuration summary.
     */
    getSummary(config: AgentDBConfig): string;
    /**
     * Export configuration as JSON string.
     */
    export(config: AgentDBConfig): string;
    /**
     * Import configuration from JSON string.
     */
    import(json: string): AgentDBConfig;
}
/**
 * Create configuration manager instance.
 */
export declare function createConfigManager(): ConfigManager;
//# sourceMappingURL=config-manager.d.ts.map