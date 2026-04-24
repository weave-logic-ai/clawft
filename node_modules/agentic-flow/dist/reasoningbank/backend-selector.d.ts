/**
 * Backend Selection Helper for ReasoningBank
 *
 * Automatically selects the optimal ReasoningBank backend based on runtime environment.
 *
 * Usage:
 * ```typescript
 * import { createOptimalReasoningBank, getRecommendedBackend } from 'agentic-flow/reasoningbank/backend-selector';
 *
 * // Automatic backend selection
 * const rb = await createOptimalReasoningBank('my-db');
 *
 * // Manual check
 * const backend = getRecommendedBackend();
 * console.log(`Using ${backend} backend`);
 * ```
 */
/**
 * Detect runtime environment and recommend optimal backend
 *
 * @returns 'nodejs' for Node.js/Deno environments, 'wasm' for browsers
 */
export declare function getRecommendedBackend(): 'nodejs' | 'wasm';
/**
 * Check if IndexedDB is available (browser environment)
 */
export declare function hasIndexedDB(): boolean;
/**
 * Check if SQLite native module is available (Node.js)
 */
export declare function hasSQLite(): boolean;
/**
 * Create ReasoningBank instance with optimal backend for current environment
 *
 * @param dbName - Database name (used differently by each backend)
 * @param options - Additional configuration options
 * @returns ReasoningBank instance using optimal backend
 *
 * @example
 * ```typescript
 * // Node.js: Creates SQLite database at .swarm/my-app.db
 * const rb = await createOptimalReasoningBank('my-app');
 *
 * // Browser: Creates IndexedDB database named 'my-app'
 * const rb = await createOptimalReasoningBank('my-app');
 * ```
 */
export declare function createOptimalReasoningBank(dbName?: string, options?: {
    /** Force specific backend (overrides auto-detection) */
    forceBackend?: 'nodejs' | 'wasm';
    /** Custom database path (Node.js only) */
    dbPath?: string;
    /** Enable verbose logging */
    verbose?: boolean;
}): Promise<any>;
/**
 * Get backend information and capabilities
 */
export declare function getBackendInfo(): {
    backend: "nodejs" | "wasm";
    environment: string;
    features: {
        persistent: boolean;
        sqlite: boolean;
        indexeddb: boolean;
        wasm: boolean;
    };
    storage: string;
};
/**
 * Validate environment and warn about limitations
 */
export declare function validateEnvironment(): {
    valid: boolean;
    warnings: string[];
    backend: 'nodejs' | 'wasm';
};
export { validateEnvironment as checkEnvironment };
//# sourceMappingURL=backend-selector.d.ts.map