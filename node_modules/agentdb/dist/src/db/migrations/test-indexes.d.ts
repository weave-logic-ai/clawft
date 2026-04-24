#!/usr/bin/env node
/**
 * AgentDB Index Performance Test
 *
 * Tests composite index performance by:
 * 1. Creating a test database with sample data
 * 2. Running queries WITHOUT composite indexes
 * 3. Applying composite index migration
 * 4. Running same queries WITH composite indexes
 * 5. Comparing performance metrics
 *
 * Usage:
 *   node test-indexes.ts
 */
import Database from 'better-sqlite3';
interface BenchmarkResult {
    query: string;
    description: string;
    timeMs: number;
    rowsReturned: number;
    indexUsed: string | null;
}
/**
 * Create test database with sample data
 */
declare function createTestDatabase(): Database.Database;
/**
 * Run query and measure performance
 */
declare function benchmarkQuery(db: Database.Database, query: string, params: any[], description: string): BenchmarkResult;
/**
 * Run benchmark suite
 */
declare function runBenchmarks(db: Database.Database): BenchmarkResult[];
export { createTestDatabase, benchmarkQuery, runBenchmarks };
//# sourceMappingURL=test-indexes.d.ts.map