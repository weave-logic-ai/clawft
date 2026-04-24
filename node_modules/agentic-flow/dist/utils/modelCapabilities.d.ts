/**
 * Model Capability Detection
 * Determines if a model supports native tool calling or requires emulation
 */
export interface ModelCapabilities {
    supportsNativeTools: boolean;
    supportsStreaming: boolean;
    contextWindow: number;
    requiresEmulation: boolean;
    emulationStrategy: 'none' | 'react' | 'prompt' | 'hybrid';
    costPerMillionTokens: number;
    provider: string;
}
/**
 * Detect capabilities of a model
 */
export declare function detectModelCapabilities(modelId: string): ModelCapabilities;
/**
 * Check if model can handle a specific number of tools
 */
export declare function canHandleToolCount(modelId: string, toolCount: number): boolean;
/**
 * Recommend optimal strategy for a model
 */
export declare function recommendStrategy(modelId: string, toolCount: number, taskComplexity: 'simple' | 'medium' | 'complex'): 'native' | 'react' | 'prompt' | 'hybrid';
/**
 * Get human-readable capability report
 */
export declare function getCapabilityReport(modelId: string): string;
//# sourceMappingURL=modelCapabilities.d.ts.map