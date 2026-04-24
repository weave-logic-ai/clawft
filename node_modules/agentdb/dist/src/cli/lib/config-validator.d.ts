/**
 * Configuration validation for AgentDB simulations
 * Validates component combinations and parameter ranges
 */
export interface ValidationResult {
    valid: boolean;
    errors: string[];
    warnings: string[];
}
export interface SimulationConfig {
    scenario?: string;
    backend?: string;
    attentionHeads?: number;
    searchStrategy?: string;
    beamWidth?: number;
    clustering?: string;
    selfHealing?: string | boolean;
    neuralFeatures?: string[];
    nodes?: number;
    dimensions?: number;
    iterations?: number;
    useOptimal?: boolean;
    [key: string]: any;
}
export declare class ConfigValidator {
    /**
     * Validate complete simulation configuration
     */
    static validate(config: SimulationConfig): ValidationResult;
    /**
     * Check if configuration matches optimal settings
     */
    static isOptimal(config: SimulationConfig): boolean;
    /**
     * Get optimal configuration for a scenario
     */
    static getOptimalConfig(scenario: string): Partial<SimulationConfig>;
    /**
     * Validate component compatibility
     */
    static validateCompatibility(config: SimulationConfig): ValidationResult;
}
//# sourceMappingURL=config-validator.d.ts.map