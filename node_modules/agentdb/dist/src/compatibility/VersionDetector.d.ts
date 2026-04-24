/**
 * VersionDetector - Automatically detects v1.x vs v2.0 API usage
 *
 * Analyzes configuration objects and method calls to determine
 * which API version is being used, enabling transparent compatibility.
 */
import type { V1Config, V2Config, CompatibilityConfig, VersionDetectionResult } from './types';
export declare class VersionDetector {
    private static readonly V1_API_METHODS;
    private static readonly V1_TO_V2_MAPPING;
    /**
     * Detect API version from configuration object
     */
    static detect(config: V1Config | V2Config | CompatibilityConfig | any, context?: string): VersionDetectionResult;
    /**
     * Check if a method name is a v1.x API
     */
    static isV1API(methodName: string): boolean;
    /**
     * Check if a method name is a v2.0 API (namespaced)
     */
    static isV2API(methodName: string): boolean;
    /**
     * Get v2.0 equivalent for v1.x API
     */
    static getAPIMapping(v1Method: string): string | undefined;
    /**
     * Get all v1.x APIs and their v2.0 equivalents
     */
    static getAllMappings(): Record<string, string>;
}
//# sourceMappingURL=VersionDetector.d.ts.map