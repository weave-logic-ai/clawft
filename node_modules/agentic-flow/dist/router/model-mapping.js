/**
 * Model ID Mapping for Multi-Provider Support
 *
 * Different providers use different model ID formats:
 * - Anthropic: "claude-sonnet-4-5-20250929" (dated releases)
 * - OpenRouter: "anthropic/claude-sonnet-4.5" (vendor/model format)
 * - AWS Bedrock: "anthropic.claude-sonnet-4-5-v2:0" (ARN-style)
 */
/**
 * Claude Model Mappings
 */
export const CLAUDE_MODELS = {
    // Claude Sonnet 4.5 (September 2025 release)
    'claude-sonnet-4.5': {
        anthropic: 'claude-sonnet-4-5-20250929',
        openrouter: 'anthropic/claude-sonnet-4.5',
        bedrock: 'anthropic.claude-sonnet-4-5-v2:0',
        canonical: 'Claude Sonnet 4.5'
    },
    // Claude Sonnet 4 (original release)
    'claude-sonnet-4': {
        anthropic: 'claude-sonnet-4-20240620',
        openrouter: 'anthropic/claude-sonnet-4',
        bedrock: 'anthropic.claude-sonnet-4-v1:0',
        canonical: 'Claude Sonnet 4'
    },
    // Claude 3.7 Sonnet
    'claude-3.7-sonnet': {
        anthropic: 'claude-3-7-sonnet-20250219',
        openrouter: 'anthropic/claude-3.7-sonnet',
        canonical: 'Claude 3.7 Sonnet'
    },
    // Claude 3.5 Sonnet (October 2024)
    'claude-3.5-sonnet': {
        anthropic: 'claude-3-5-sonnet-20241022',
        openrouter: 'anthropic/claude-3.5-sonnet-20241022',
        bedrock: 'anthropic.claude-3-5-sonnet-20241022-v2:0',
        canonical: 'Claude 3.5 Sonnet'
    },
    // Claude 3.5 Haiku
    'claude-3.5-haiku': {
        anthropic: 'claude-3-5-haiku-20241022',
        openrouter: 'anthropic/claude-3.5-haiku-20241022',
        canonical: 'Claude 3.5 Haiku'
    },
    // Claude Opus 4.1
    'claude-opus-4.1': {
        anthropic: 'claude-opus-4-1-20250514',
        openrouter: 'anthropic/claude-opus-4.1',
        canonical: 'Claude Opus 4.1'
    }
};
/**
 * Map a model ID from one provider format to another
 */
export function mapModelId(modelId, targetProvider) {
    // If already in correct format, return as-is
    if (targetProvider === 'anthropic' && modelId.startsWith('claude-')) {
        // Check if it's already an Anthropic API ID (has date like 20250929)
        if (/claude-.*-\d{8}/.test(modelId)) {
            return modelId;
        }
    }
    if (targetProvider === 'openrouter' && modelId.startsWith('anthropic/')) {
        return modelId;
    }
    // Try to find canonical mapping
    for (const [canonical, mapping] of Object.entries(CLAUDE_MODELS)) {
        if (modelId === mapping.anthropic ||
            modelId === mapping.openrouter ||
            modelId === mapping.bedrock ||
            modelId === canonical) {
            const mapped = mapping[targetProvider];
            if (mapped) {
                return mapped;
            }
        }
    }
    // If no mapping found, try to convert format
    if (targetProvider === 'openrouter') {
        // Convert Anthropic format to OpenRouter format
        // claude-sonnet-4-5-20250929 -> anthropic/claude-sonnet-4.5
        if (modelId.startsWith('claude-')) {
            const withoutDate = modelId.replace(/-\d{8}$/, '');
            const parts = withoutDate.split('-');
            if (parts.length >= 3) {
                const family = parts[0]; // claude
                const tier = parts[1]; // sonnet, opus, haiku
                const version = parts.slice(2).join('.'); // 4.5 or 3.5
                return `anthropic/${family}-${tier}-${version}`;
            }
        }
    }
    else if (targetProvider === 'anthropic') {
        // Convert OpenRouter format to Anthropic format
        // anthropic/claude-sonnet-4.5 -> claude-sonnet-4-5-20250929
        if (modelId.startsWith('anthropic/')) {
            const withoutPrefix = modelId.replace('anthropic/', '');
            // Look up in mappings by OpenRouter ID
            for (const mapping of Object.values(CLAUDE_MODELS)) {
                if (mapping.openrouter === modelId) {
                    return mapping.anthropic;
                }
            }
        }
    }
    // No conversion possible, return original
    console.warn(`⚠️  No model mapping found for '${modelId}' to ${targetProvider}, using original ID`);
    return modelId;
}
/**
 * Get human-readable model name
 */
export function getModelName(modelId) {
    for (const mapping of Object.values(CLAUDE_MODELS)) {
        if (modelId === mapping.anthropic ||
            modelId === mapping.openrouter ||
            modelId === mapping.bedrock) {
            return mapping.canonical;
        }
    }
    return modelId;
}
/**
 * List all available model IDs for a provider
 */
export function listModels(provider) {
    return Object.values(CLAUDE_MODELS)
        .map(m => m[provider])
        .filter((id) => id !== undefined);
}
//# sourceMappingURL=model-mapping.js.map