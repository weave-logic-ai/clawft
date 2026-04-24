/**
 * Backend Factory - Automatic Backend Detection and Selection
 *
 * Detects available vector backends and creates appropriate instances.
 * Priority: RuVector (native/WASM) > HNSWLib (Node.js)
 *
 * Features:
 * - Automatic detection of @ruvector packages
 * - Native vs WASM detection for RuVector
 * - GNN and Graph capabilities detection
 * - Graceful fallback to HNSWLib
 * - Clear error messages for missing dependencies
 */
import type { VectorBackend, VectorConfig } from './VectorBackend.js';
export type BackendType = 'auto' | 'ruvector' | 'hnswlib';
export interface BackendDetection {
    available: 'ruvector' | 'hnswlib' | 'none';
    ruvector: {
        core: boolean;
        gnn: boolean;
        graph: boolean;
        native: boolean;
    };
    hnswlib: boolean;
}
/**
 * Detect available vector backends
 */
export declare function detectBackends(): Promise<BackendDetection>;
/**
 * Create vector backend with automatic detection
 *
 * @param type - Backend type: 'auto', 'ruvector', or 'hnswlib'
 * @param config - Vector configuration
 * @returns Initialized VectorBackend instance
 */
export declare function createBackend(type: BackendType, config: VectorConfig): Promise<VectorBackend>;
/**
 * Get recommended backend type based on environment
 */
export declare function getRecommendedBackend(): Promise<BackendType>;
/**
 * Check if a specific backend is available
 */
export declare function isBackendAvailable(backend: 'ruvector' | 'hnswlib'): Promise<boolean>;
/**
 * Get installation instructions for a backend
 */
export declare function getInstallCommand(backend: 'ruvector' | 'hnswlib'): string;
//# sourceMappingURL=factory.d.ts.map