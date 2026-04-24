/**
 * Attention Configuration Management
 * Handles loading, saving, and validating attention mechanism configurations
 */
import * as fs from 'fs/promises';
import * as path from 'path';
/**
 * Default attention configuration
 */
export const DEFAULT_ATTENTION_CONFIG = {
    defaultMechanism: 'flash',
    mechanisms: {
        flash: {
            enabled: true,
            heads: 8,
            dimension: 384,
            blockSize: 64,
        },
        hyperbolic: {
            enabled: true,
            curvature: -1.0,
            heads: 8,
            dimension: 384,
        },
        sparse: {
            enabled: true,
            sparsity: 0.9,
            heads: 8,
            dimension: 384,
        },
        linear: {
            enabled: true,
            kernelSize: 32,
            heads: 8,
            dimension: 384,
        },
        performer: {
            enabled: true,
            randomFeatures: 256,
            heads: 8,
            dimension: 384,
        },
    },
    featureFlags: {
        enableBenchmarking: true,
        enableOptimization: true,
        cacheResults: true,
    },
};
/**
 * Load attention configuration from file
 */
export async function loadAttentionConfig(configPath) {
    const defaultPath = path.join(process.cwd(), '.agentdb', 'attention-config.json');
    const filePath = configPath || defaultPath;
    try {
        const data = await fs.readFile(filePath, 'utf-8');
        const config = JSON.parse(data);
        return validateConfig(config);
    }
    catch (error) {
        if (error.code === 'ENOENT') {
            // Config file doesn't exist, return default
            return DEFAULT_ATTENTION_CONFIG;
        }
        throw new Error(`Failed to load attention config: ${error.message}`);
    }
}
/**
 * Save attention configuration to file
 */
export async function saveAttentionConfig(config, configPath) {
    const defaultPath = path.join(process.cwd(), '.agentdb', 'attention-config.json');
    const filePath = configPath || defaultPath;
    // Ensure directory exists
    await fs.mkdir(path.dirname(filePath), { recursive: true });
    // Validate before saving
    const validConfig = validateConfig(config);
    // Save to file
    await fs.writeFile(filePath, JSON.stringify(validConfig, null, 2));
}
/**
 * Validate attention configuration
 */
export function validateConfig(config) {
    if (!config || typeof config !== 'object') {
        throw new Error('Invalid configuration: must be an object');
    }
    // Validate default mechanism
    if (!config.defaultMechanism) {
        config.defaultMechanism = 'flash';
    }
    const validMechanisms = ['flash', 'hyperbolic', 'sparse', 'linear', 'performer'];
    if (!validMechanisms.includes(config.defaultMechanism)) {
        throw new Error(`Invalid default mechanism: ${config.defaultMechanism}. Must be one of: ${validMechanisms.join(', ')}`);
    }
    // Validate mechanisms
    if (!config.mechanisms || typeof config.mechanisms !== 'object') {
        config.mechanisms = DEFAULT_ATTENTION_CONFIG.mechanisms;
    }
    // Validate each mechanism
    for (const mechanismName of validMechanisms) {
        if (!config.mechanisms[mechanismName]) {
            config.mechanisms[mechanismName] = DEFAULT_ATTENTION_CONFIG.mechanisms[mechanismName];
            continue;
        }
        const mechanism = config.mechanisms[mechanismName];
        // Validate common fields
        if (typeof mechanism.enabled !== 'boolean') {
            mechanism.enabled = true;
        }
        if (!Number.isInteger(mechanism.heads) || mechanism.heads < 1 || mechanism.heads > 32) {
            throw new Error(`Invalid heads for ${mechanismName}: must be an integer between 1 and 32`);
        }
        if (!Number.isInteger(mechanism.dimension) || mechanism.dimension < 64 || mechanism.dimension > 2048) {
            throw new Error(`Invalid dimension for ${mechanismName}: must be an integer between 64 and 2048`);
        }
        // Validate mechanism-specific fields
        switch (mechanismName) {
            case 'flash':
                if (!Number.isInteger(mechanism.blockSize) || mechanism.blockSize < 16 || mechanism.blockSize > 256) {
                    throw new Error('Invalid blockSize for flash: must be an integer between 16 and 256');
                }
                break;
            case 'hyperbolic':
                if (typeof mechanism.curvature !== 'number' || mechanism.curvature >= 0) {
                    throw new Error('Invalid curvature for hyperbolic: must be a negative number');
                }
                break;
            case 'sparse':
                if (typeof mechanism.sparsity !== 'number' || mechanism.sparsity < 0 || mechanism.sparsity > 1) {
                    throw new Error('Invalid sparsity: must be a number between 0 and 1');
                }
                break;
            case 'linear':
                if (!Number.isInteger(mechanism.kernelSize) || mechanism.kernelSize < 8 || mechanism.kernelSize > 128) {
                    throw new Error('Invalid kernelSize for linear: must be an integer between 8 and 128');
                }
                break;
            case 'performer':
                if (!Number.isInteger(mechanism.randomFeatures) || mechanism.randomFeatures < 64 || mechanism.randomFeatures > 1024) {
                    throw new Error('Invalid randomFeatures for performer: must be an integer between 64 and 1024');
                }
                break;
        }
    }
    // Validate feature flags
    if (!config.featureFlags || typeof config.featureFlags !== 'object') {
        config.featureFlags = DEFAULT_ATTENTION_CONFIG.featureFlags;
    }
    if (typeof config.featureFlags.enableBenchmarking !== 'boolean') {
        config.featureFlags.enableBenchmarking = true;
    }
    if (typeof config.featureFlags.enableOptimization !== 'boolean') {
        config.featureFlags.enableOptimization = true;
    }
    if (typeof config.featureFlags.cacheResults !== 'boolean') {
        config.featureFlags.cacheResults = true;
    }
    return config;
}
/**
 * Update a specific mechanism configuration
 */
export async function updateMechanismConfig(mechanismName, updates, configPath) {
    const config = await loadAttentionConfig(configPath);
    if (!(mechanismName in config.mechanisms)) {
        throw new Error(`Unknown mechanism: ${mechanismName}`);
    }
    // Apply updates
    config.mechanisms[mechanismName] = {
        ...config.mechanisms[mechanismName],
        ...updates,
    };
    // Validate and save
    const validConfig = validateConfig(config);
    await saveAttentionConfig(validConfig, configPath);
    return validConfig;
}
/**
 * Enable/disable a mechanism
 */
export async function toggleMechanism(mechanismName, enabled, configPath) {
    return updateMechanismConfig(mechanismName, { enabled }, configPath);
}
/**
 * Set default mechanism
 */
export async function setDefaultMechanism(mechanismName, configPath) {
    const config = await loadAttentionConfig(configPath);
    const validMechanisms = ['flash', 'hyperbolic', 'sparse', 'linear', 'performer'];
    if (!validMechanisms.includes(mechanismName)) {
        throw new Error(`Invalid mechanism: ${mechanismName}. Must be one of: ${validMechanisms.join(', ')}`);
    }
    config.defaultMechanism = mechanismName;
    const validConfig = validateConfig(config);
    await saveAttentionConfig(validConfig, configPath);
    return validConfig;
}
/**
 * Get configuration for a specific mechanism
 */
export async function getMechanismConfig(mechanismName, configPath) {
    const config = await loadAttentionConfig(configPath);
    if (!(mechanismName in config.mechanisms)) {
        throw new Error(`Unknown mechanism: ${mechanismName}`);
    }
    return config.mechanisms[mechanismName];
}
/**
 * Reset configuration to defaults
 */
export async function resetConfig(configPath) {
    await saveAttentionConfig(DEFAULT_ATTENTION_CONFIG, configPath);
    return DEFAULT_ATTENTION_CONFIG;
}
/**
 * Export configuration as JSON string
 */
export function exportConfig(config) {
    return JSON.stringify(config, null, 2);
}
/**
 * Import configuration from JSON string
 */
export function importConfig(jsonString) {
    const config = JSON.parse(jsonString);
    return validateConfig(config);
}
//# sourceMappingURL=attention-config.js.map