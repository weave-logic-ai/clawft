/**
 * Test Database Factory - Uses better-sqlite3 when available for testing
 *
 * Integration tests need to create large datasets and run many queries,
 * which can exceed sql.js WASM memory limit (64MB). This factory attempts
 * to use better-sqlite3 for tests, falling back to sql.js if unavailable.
 */

import type { Database } from 'better-sqlite3';

let betterSqlite3: any = null;

/**
 * Try to load better-sqlite3 (optional dependency)
 */
async function loadBetterSqlite3(): Promise<any> {
  if (betterSqlite3) return betterSqlite3;

  try {
    betterSqlite3 = (await import('better-sqlite3')).default;
    console.log('✅ Using better-sqlite3 for tests (no memory limit)');
    return betterSqlite3;
  } catch (error) {
    console.warn('⚠️  better-sqlite3 not available, falling back to sql.js (64MB memory limit)');
    console.warn('   Install with: npm install --save-dev better-sqlite3');
    return null;
  }
}

/**
 * Wrap better-sqlite3 to add sql.js-compatible save() method
 */
function wrapBetterSqlite3(db: any): any {
  // Add sql.js-compatible save() method (no-op for better-sqlite3)
  if (!db.save) {
    db.save = function() {
      // better-sqlite3 auto-saves, so this is a no-op
      return;
    };
  }
  return db;
}

/**
 * Create database for testing - prefers better-sqlite3, falls back to sql.js
 */
export async function createTestDatabase(filename: string, options?: any): Promise<Database> {
  const BetterSqlite3 = await loadBetterSqlite3();

  if (BetterSqlite3) {
    // Use better-sqlite3 (native, no memory limit)
    const db = new BetterSqlite3(filename, options);
    return wrapBetterSqlite3(db);
  } else {
    // Fall back to sql.js (WASM, 64MB limit)
    const { createDatabase } = await import('./db-fallback.js');
    return createDatabase(filename, options);
  }
}
