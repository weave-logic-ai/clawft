/**
 * Attention Configuration Management
 * Handles loading, saving, and validating attention mechanism configurations
 */
export interface AttentionMechanismConfig {
    enabled: boolean;
    heads: number;
    dimension: number;
    [key: string]: any;
}
export interface AttentionConfig {
    defaultMechanism: string;
    mechanisms: {
        flash: AttentionMechanismConfig & {
            blockSize: number;
        };
        hyperbolic: AttentionMechanismConfig & {
            curvature: number;
        };
        sparse: AttentionMechanismConfig & {
            sparsity: number;
        };
        linear: AttentionMechanismConfig & {
            kernelSize: number;
        };
        performer: AttentionMechanismConfig & {
            randomFeatures: number;
        };
    };
    featureFlags: {
        enableBenchmarking: boolean;
        enableOptimization: boolean;
        cacheResults: boolean;
    };
}
/**
 * Default attention configuration
 */
export declare const DEFAULT_ATTENTION_CONFIG: AttentionConfig;
/**
 * Load attention configuration from file
 */
export declare function loadAttentionConfig(configPath?: string): Promise<AttentionConfig>;
/**
 * Save attention configuration to file
 */
export declare function saveAttentionConfig(config: AttentionConfig, configPath?: string): Promise<void>;
/**
 * Validate attention configuration
 */
export declare function validateConfig(config: any): AttentionConfig;
/**
 * Update a specific mechanism configuration
 */
export declare function updateMechanismConfig(mechanismName: string, updates: Partial<AttentionMechanismConfig>, configPath?: string): Promise<AttentionConfig>;
/**
 * Enable/disable a mechanism
 */
export declare function toggleMechanism(mechanismName: string, enabled: boolean, configPath?: string): Promise<AttentionConfig>;
/**
 * Set default mechanism
 */
export declare function setDefaultMechanism(mechanismName: string, configPath?: string): Promise<AttentionConfig>;
/**
 * Get configuration for a specific mechanism
 */
export declare function getMechanismConfig(mechanismName: string, configPath?: string): Promise<AttentionMechanismConfig>;
/**
 * Reset configuration to defaults
 */
export declare function resetConfig(configPath?: string): Promise<AttentionConfig>;
/**
 * Export configuration as JSON string
 */
export declare function exportConfig(config: AttentionConfig): string;
/**
 * Import configuration from JSON string
 */
export declare function importConfig(jsonString: string): AttentionConfig;
//# sourceMappingURL=attention-config.d.ts.map