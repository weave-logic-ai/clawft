/**
 * Backend Detection - Auto-detect available vector backends
 *
 * Detection priority:
 * 1. RuVector (@ruvector/core) - preferred for performance
 * 2. HNSWLib (hnswlib-node) - stable fallback
 *
 * Additional features detected:
 * - @ruvector/gnn - GNN learning capabilities
 * - @ruvector/graph-node - Graph database capabilities
 */
/**
 * Backend type identifier
 */
export type BackendType = 'ruvector' | 'hnswlib' | 'auto';
/**
 * Platform information
 */
export interface PlatformInfo {
    /** Operating system */
    platform: NodeJS.Platform;
    /** CPU architecture */
    arch: string;
    /** Combined platform identifier (e.g., 'linux-x64', 'darwin-arm64') */
    combined: string;
}
/**
 * Backend detection result
 */
export interface DetectionResult {
    /** Detected backend type */
    backend: 'ruvector' | 'hnswlib';
    /** Available feature flags */
    features: {
        /** GNN learning available */
        gnn: boolean;
        /** Graph database available */
        graph: boolean;
        /** Compression available */
        compression: boolean;
    };
    /** Platform information */
    platform: PlatformInfo;
    /** Whether native bindings are available (vs WASM fallback) */
    native: boolean;
    /** Version information */
    versions?: {
        core?: string;
        gnn?: string;
        graph?: string;
    };
}
/**
 * Detect available vector backend and features
 *
 * @returns Detection result with backend type and available features
 */
export declare function detectBackend(): Promise<DetectionResult>;
/**
 * Validate requested backend is available
 *
 * @param requested - Requested backend type
 * @param detected - Detected backend from auto-detection
 * @throws Error if requested backend is not available
 */
export declare function validateBackend(requested: BackendType, detected: DetectionResult): void;
/**
 * Get recommended backend for a given use case
 *
 * @param useCase - Use case identifier
 * @returns Recommended backend type
 */
export declare function getRecommendedBackend(useCase: string): BackendType;
/**
 * Format detection result for display
 *
 * @param result - Detection result
 * @returns Formatted string for console output
 */
export declare function formatDetectionResult(result: DetectionResult): string;
//# sourceMappingURL=detector.d.ts.map