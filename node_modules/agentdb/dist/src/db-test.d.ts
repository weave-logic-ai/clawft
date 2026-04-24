/**
 * Test Database Factory - Uses better-sqlite3 when available for testing
 *
 * Integration tests need to create large datasets and run many queries,
 * which can exceed sql.js WASM memory limit (64MB). This factory attempts
 * to use better-sqlite3 for tests, falling back to sql.js if unavailable.
 */
import type { Database } from 'better-sqlite3';
/**
 * Create database for testing - prefers better-sqlite3, falls back to sql.js
 */
export declare function createTestDatabase(filename: string, options?: any): Promise<Database>;
//# sourceMappingURL=db-test.d.ts.map