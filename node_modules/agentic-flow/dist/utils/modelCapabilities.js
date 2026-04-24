/**
 * Model Capability Detection
 * Determines if a model supports native tool calling or requires emulation
 */
/**
 * Database of known model capabilities
 * Updated periodically as new models are released
 */
const MODEL_CAPABILITIES = {
    // OpenRouter - Native Tool Support
    'deepseek/deepseek-chat': {
        supportsNativeTools: true,
        contextWindow: 128000,
        requiresEmulation: false,
        emulationStrategy: 'none',
        costPerMillionTokens: 0.15
    },
    'deepseek/deepseek-chat-v3': {
        supportsNativeTools: true,
        contextWindow: 128000,
        requiresEmulation: false,
        emulationStrategy: 'none',
        costPerMillionTokens: 0.15
    },
    'meta-llama/llama-3.3-70b-instruct': {
        supportsNativeTools: true,
        contextWindow: 128000,
        requiresEmulation: false,
        emulationStrategy: 'none',
        costPerMillionTokens: 0.40
    },
    'qwen/qwen-2.5-coder-32b-instruct': {
        supportsNativeTools: true,
        contextWindow: 128000,
        requiresEmulation: false,
        emulationStrategy: 'none',
        costPerMillionTokens: 0.23
    },
    'mistralai/mistral-small-3.1-24b-instruct': {
        supportsNativeTools: true,
        contextWindow: 128000,
        requiresEmulation: false,
        emulationStrategy: 'none',
        costPerMillionTokens: 0.30
    },
    // OpenAI Models (via OpenRouter/Requesty)
    'openai/gpt-4o': {
        supportsNativeTools: true,
        contextWindow: 128000,
        requiresEmulation: false,
        emulationStrategy: 'none',
        costPerMillionTokens: 2.50
    },
    'openai/gpt-4o-mini': {
        supportsNativeTools: true,
        contextWindow: 128000,
        requiresEmulation: false,
        emulationStrategy: 'none',
        costPerMillionTokens: 0.15
    },
    'openai/gpt-4-turbo': {
        supportsNativeTools: true,
        contextWindow: 128000,
        requiresEmulation: false,
        emulationStrategy: 'none',
        costPerMillionTokens: 10.00
    },
    // OpenRouter - No Native Tool Support (Require Emulation)
    'mistralai/mistral-7b-instruct': {
        supportsNativeTools: false,
        contextWindow: 32000,
        requiresEmulation: true,
        emulationStrategy: 'react', // Best for this model
        costPerMillionTokens: 0.07
    },
    'meta-llama/llama-2-13b-chat': {
        supportsNativeTools: false,
        contextWindow: 4096,
        requiresEmulation: true,
        emulationStrategy: 'prompt', // Simple prompting works best
        costPerMillionTokens: 0.05
    },
    'google/gemma-7b-it': {
        supportsNativeTools: false,
        contextWindow: 8192,
        requiresEmulation: true,
        emulationStrategy: 'prompt',
        costPerMillionTokens: 0.07
    },
    'thudm/glm-4-9b:free': {
        supportsNativeTools: false, // Free tier limited
        contextWindow: 32000,
        requiresEmulation: true,
        emulationStrategy: 'react',
        costPerMillionTokens: 0.0 // FREE
    },
    // Anthropic - Native Tool Support
    'claude-3-5-sonnet-20241022': {
        supportsNativeTools: true,
        contextWindow: 200000,
        requiresEmulation: false,
        emulationStrategy: 'none',
        costPerMillionTokens: 3.0
    },
    'claude-3-5-haiku-20241022': {
        supportsNativeTools: true,
        contextWindow: 200000,
        requiresEmulation: false,
        emulationStrategy: 'none',
        costPerMillionTokens: 0.80
    },
    // Gemini - Native Tool Support
    'gemini-2.0-flash-exp': {
        supportsNativeTools: true,
        contextWindow: 1000000,
        requiresEmulation: false,
        emulationStrategy: 'none',
        costPerMillionTokens: 0.0 // FREE tier
    },
    // OpenAI - Native Tool Support
    'gpt-4o': {
        supportsNativeTools: true,
        contextWindow: 128000,
        requiresEmulation: false,
        emulationStrategy: 'none',
        costPerMillionTokens: 2.50
    },
    'gpt-4o-mini': {
        supportsNativeTools: true,
        contextWindow: 128000,
        requiresEmulation: false,
        emulationStrategy: 'none',
        costPerMillionTokens: 0.15
    },
    'gpt-3.5-turbo': {
        supportsNativeTools: true,
        contextWindow: 16385,
        requiresEmulation: false,
        emulationStrategy: 'none',
        costPerMillionTokens: 0.50
    }
};
/**
 * Pattern matching for model families
 */
const MODEL_PATTERNS = [
    {
        pattern: /^claude-3/,
        capabilities: { supportsNativeTools: true, requiresEmulation: false }
    },
    {
        pattern: /^gpt-(4|3\.5)/,
        capabilities: { supportsNativeTools: true, requiresEmulation: false }
    },
    {
        pattern: /^gemini-/,
        capabilities: { supportsNativeTools: true, requiresEmulation: false }
    },
    {
        pattern: /deepseek\/deepseek-(chat|coder)/,
        capabilities: { supportsNativeTools: true, requiresEmulation: false }
    },
    {
        pattern: /llama-3\.(3|2|1)-.*instruct/,
        capabilities: { supportsNativeTools: true, requiresEmulation: false }
    },
    {
        pattern: /qwen.*2\.5/,
        capabilities: { supportsNativeTools: true, requiresEmulation: false }
    },
    {
        pattern: /mistral.*large|mistral.*small/,
        capabilities: { supportsNativeTools: true, requiresEmulation: false }
    },
    // Older models typically don't support tools
    {
        pattern: /llama-2|mistral-7b|gemma-.*7b/,
        capabilities: { supportsNativeTools: false, requiresEmulation: true, emulationStrategy: 'react' }
    }
];
/**
 * Detect capabilities of a model
 */
export function detectModelCapabilities(modelId) {
    // Direct lookup
    if (MODEL_CAPABILITIES[modelId]) {
        return {
            supportsNativeTools: false,
            supportsStreaming: true,
            contextWindow: 4096,
            requiresEmulation: true,
            emulationStrategy: 'prompt',
            costPerMillionTokens: 0.5,
            provider: 'unknown',
            ...MODEL_CAPABILITIES[modelId]
        };
    }
    // Pattern matching
    for (const { pattern, capabilities } of MODEL_PATTERNS) {
        if (pattern.test(modelId)) {
            return {
                supportsNativeTools: false,
                supportsStreaming: true,
                contextWindow: 4096,
                requiresEmulation: true,
                emulationStrategy: 'prompt',
                costPerMillionTokens: 0.5,
                provider: 'unknown',
                ...capabilities
            };
        }
    }
    // Unknown model - assume needs emulation (conservative)
    console.warn(`⚠️  Unknown model: ${modelId}. Assuming no native tool support.`);
    return {
        supportsNativeTools: false,
        supportsStreaming: true,
        contextWindow: 4096,
        requiresEmulation: true,
        emulationStrategy: 'prompt',
        costPerMillionTokens: 0.5,
        provider: 'unknown'
    };
}
/**
 * Check if model can handle a specific number of tools
 */
export function canHandleToolCount(modelId, toolCount) {
    const capabilities = detectModelCapabilities(modelId);
    // Estimate: Each tool ~100 tokens of schema
    const estimatedTokens = toolCount * 100;
    // Need at least 20% of context window free for actual conversation
    const maxToolTokens = capabilities.contextWindow * 0.8;
    return estimatedTokens <= maxToolTokens;
}
/**
 * Recommend optimal strategy for a model
 */
export function recommendStrategy(modelId, toolCount, taskComplexity) {
    const capabilities = detectModelCapabilities(modelId);
    if (capabilities.supportsNativeTools) {
        return 'native';
    }
    // For emulation, choose based on context and complexity
    if (toolCount > 10 || taskComplexity === 'complex') {
        return 'react'; // More robust for complex scenarios
    }
    if (capabilities.contextWindow < 8000) {
        return 'prompt'; // Simpler for small context windows
    }
    return 'react'; // Default to ReAct pattern
}
/**
 * Get human-readable capability report
 */
export function getCapabilityReport(modelId) {
    const cap = detectModelCapabilities(modelId);
    let report = `\n📊 Model Capabilities: ${modelId}\n`;
    report += `━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n`;
    report += `Native Tool Support:  ${cap.supportsNativeTools ? '✅ Yes' : '❌ No'}\n`;
    report += `Context Window:       ${cap.contextWindow.toLocaleString()} tokens\n`;
    report += `Streaming:            ${cap.supportsStreaming ? '✅ Yes' : '❌ No'}\n`;
    report += `Cost per 1M tokens:   $${cap.costPerMillionTokens}\n`;
    if (cap.requiresEmulation) {
        report += `\n⚙️  Emulation Required\n`;
        report += `Recommended Strategy: ${cap.emulationStrategy.toUpperCase()}\n`;
        report += `\nNote: This model will use prompt-based tool emulation.\n`;
        report += `Expect 70-85% reliability vs 95%+ for native tool support.\n`;
    }
    else {
        report += `\n✅ No Emulation Needed\n`;
        report += `This model natively supports function calling.\n`;
    }
    report += `━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n`;
    return report;
}
//# sourceMappingURL=modelCapabilities.js.map