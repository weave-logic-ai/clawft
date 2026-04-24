/**
 * Model ID Mapping for Multi-Provider Support
 *
 * Different providers use different model ID formats:
 * - Anthropic: "claude-sonnet-4-5-20250929" (dated releases)
 * - OpenRouter: "anthropic/claude-sonnet-4.5" (vendor/model format)
 * - AWS Bedrock: "anthropic.claude-sonnet-4-5-v2:0" (ARN-style)
 */
export interface ModelMapping {
    anthropic: string;
    openrouter: string;
    bedrock?: string;
    canonical: string;
}
/**
 * Claude Model Mappings
 */
export declare const CLAUDE_MODELS: Record<string, ModelMapping>;
/**
 * Map a model ID from one provider format to another
 */
export declare function mapModelId(modelId: string, targetProvider: 'anthropic' | 'openrouter' | 'bedrock'): string;
/**
 * Get human-readable model name
 */
export declare function getModelName(modelId: string): string;
/**
 * List all available model IDs for a provider
 */
export declare function listModels(provider: 'anthropic' | 'openrouter' | 'bedrock'): string[];
//# sourceMappingURL=model-mapping.d.ts.map