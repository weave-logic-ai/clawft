/**
 * Database System using sql.js (WASM SQLite)
 * Pure JavaScript implementation with NO build dependencies
 *
 * SECURITY: Fixed SQL injection vulnerabilities:
 * - PRAGMA commands validated against whitelist
 * - Removed eval() usage (replaced with async import)
 */
/**
 * Get sql.js database implementation (ONLY sql.js, no better-sqlite3)
 */
export declare function getDatabaseImplementation(): Promise<any>;
/**
 * Create a database instance using sql.js
 */
export declare function createDatabase(filename: string, options?: any): Promise<any>;
/**
 * Get information about current database implementation
 */
export declare function getDatabaseInfo(): {
    implementation: string;
    isNative: boolean;
    performance: 'high' | 'medium' | 'low';
    requiresBuildTools: boolean;
};
//# sourceMappingURL=db-fallback.d.ts.map