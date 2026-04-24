/**
 * Database System using sql.js (WASM SQLite)
 * Pure JavaScript implementation with NO build dependencies
 *
 * SECURITY: Fixed SQL injection vulnerabilities:
 * - PRAGMA commands validated against whitelist
 * - Removed eval() usage (replaced with async import)
 */

import { validatePragmaCommand, ValidationError } from './security/input-validation.js';
import * as fs from 'fs';
import * as path from 'path';

// Type-only for compatibility
type Database = any;

let sqlJsWrapper: any = null;

/**
 * Get sql.js database implementation (ONLY sql.js, no better-sqlite3)
 */
export async function getDatabaseImplementation(): Promise<any> {
  // Return cached wrapper
  if (sqlJsWrapper) {
    return sqlJsWrapper;
  }

  try {
    console.log('✅ Using sql.js (WASM SQLite, no build tools required)');

    // sql.js requires async initialization
    const mod = await import('sql.js');
    const SQL = await mod.default();

    // Create database wrapper
    sqlJsWrapper = createSqlJsWrapper(SQL);

    return sqlJsWrapper;
  } catch (error) {
    console.error('❌ Failed to initialize sql.js:', (error as Error).message);
    throw new Error(
      'Failed to initialize SQLite. Please ensure sql.js is installed:\n' +
      'npm install sql.js'
    );
  }
}

/**
 * Create a better-sqlite3 compatible wrapper around sql.js
 * This allows AgentDB to work (with reduced performance) without native compilation
 */
function createSqlJsWrapper(SQL: any) {
  return class SqlJsDatabase {
    private db: any;
    private filename: string;
    private activeStatements: Map<number, any> = new Map();
    private statementCounter: number = 0;
    private intervalId: NodeJS.Timeout | null = null;

    constructor(filename: string, options?: any) {
      this.filename = filename;

      // In-memory database
      if (filename === ':memory:') {
        this.db = new SQL.Database();
      } else {
        // File-based database - use safe fs module (no eval)
        try {
          if (fs.existsSync(filename)) {
            const buffer = fs.readFileSync(filename);
            this.db = new SQL.Database(buffer);
          } else {
            this.db = new SQL.Database();
          }
        } catch (error) {
          console.warn('⚠️  Could not read database file:', (error as Error).message);
          this.db = new SQL.Database();
        }
      }

      // Warn if too many active statements (memory leak detection)
      this.intervalId = setInterval(() => {
        if (this.activeStatements.size > 50) {
          console.warn(`⚠️  Detected ${this.activeStatements.size} active SQL statements - possible memory leak`);
        }
      }, 10000);
    }

    prepare(sql: string) {
      const stmt = this.db.prepare(sql);
      let isFinalized = false;
      const stmtId = ++this.statementCounter;

      // Track active statement
      this.activeStatements.set(stmtId, stmt);

      return {
        run: (...params: any[]) => {
          if (isFinalized) throw new Error('Statement already finalized');
          try {
            stmt.bind(params);
            stmt.step();
            stmt.reset();

            return {
              changes: this.db.getRowsModified(),
              lastInsertRowid: this.db.exec('SELECT last_insert_rowid()')[0]?.values[0]?.[0] || 0
            };
          } catch (error) {
            // Auto-free on error to prevent memory leak
            if (!isFinalized) {
              stmt.free();
              isFinalized = true;
              this.activeStatements.delete(stmtId);
            }
            throw error;
          }
        },

        get: (...params: any[]) => {
          if (isFinalized) throw new Error('Statement already finalized');
          try {
            stmt.bind(params);
            const hasRow = stmt.step();

            if (!hasRow) {
              stmt.reset();
              return undefined;
            }

            const columns = stmt.getColumnNames();
            const values = stmt.get();
            stmt.reset();

            const result: any = {};
            columns.forEach((col: string, idx: number) => {
              result[col] = values[idx];
            });

            return result;
          } catch (error) {
            // Auto-free on error to prevent memory leak
            if (!isFinalized) {
              stmt.free();
              isFinalized = true;
              this.activeStatements.delete(stmtId);
            }
            throw error;
          }
        },

        all: (...params: any[]) => {
          if (isFinalized) throw new Error('Statement already finalized');
          try {
            stmt.bind(params);
            const results: any[] = [];

            while (stmt.step()) {
              const columns = stmt.getColumnNames();
              const values = stmt.get();

              const result: any = {};
              columns.forEach((col: string, idx: number) => {
                result[col] = values[idx];
              });

              results.push(result);
            }

            stmt.reset();
            return results;
          } catch (error) {
            // Auto-free on error to prevent memory leak
            if (!isFinalized) {
              stmt.free();
              isFinalized = true;
              this.activeStatements.delete(stmtId);
            }
            throw error;
          }
        },

        finalize: () => {
          if (!isFinalized) {
            stmt.free();
            isFinalized = true;
            this.activeStatements.delete(stmtId);
          }
        }
      };
    }

    exec(sql: string) {
      return this.db.exec(sql);
    }

    save() {
      // Save to file if needed
      if (this.filename !== ':memory:') {
        try {
          // Create parent directories if they don't exist
          const dir = path.dirname(this.filename);
          if (!fs.existsSync(dir)) {
            fs.mkdirSync(dir, { recursive: true });
          }

          const data = this.db.export();
          fs.writeFileSync(this.filename, Buffer.from(data));
        } catch (error) {
          console.error('❌ Could not save database to file:', (error as Error).message);
          throw error;
        }
      }
    }

    close() {
      // Clear interval timer
      if (this.intervalId) {
        clearInterval(this.intervalId);
        this.intervalId = null;
      }

      // Free all active statements to prevent memory leaks
      for (const [stmtId, stmt] of this.activeStatements.entries()) {
        try {
          stmt.free();
        } catch (e) {
          // Statement may already be freed
        }
      }
      this.activeStatements.clear();

      // Save to file before closing
      this.save();
      this.db.close();
    }

    pragma(pragma: string, options?: any) {
      try {
        // SECURITY: Validate PRAGMA command against whitelist to prevent SQL injection
        const validatedPragma = validatePragmaCommand(pragma);

        // Execute validated PRAGMA
        const result = this.db.exec(`PRAGMA ${validatedPragma}`);
        return result[0]?.values[0]?.[0];
      } catch (error) {
        if (error instanceof ValidationError) {
          console.error(`❌ Invalid PRAGMA command: ${error.message}`);
          throw error;
        }
        throw error;
      }
    }

    transaction(fn: () => any) {
      // Return a function that executes the transaction when called
      // This matches better-sqlite3 API where transaction() returns a callable function
      return () => {
        try {
          this.db.exec('BEGIN TRANSACTION');
          const result = fn();
          this.db.exec('COMMIT');
          return result;
        } catch (error) {
          this.db.exec('ROLLBACK');
          throw error;
        }
      };
    }
  };
}

/**
 * Create a database instance using sql.js
 */
export async function createDatabase(filename: string, options?: any): Promise<any> {
  const DatabaseImpl = await getDatabaseImplementation();
  return new DatabaseImpl(filename, options);
}

/**
 * Get information about current database implementation
 */
export function getDatabaseInfo(): {
  implementation: string;
  isNative: boolean;
  performance: 'high' | 'medium' | 'low';
  requiresBuildTools: boolean;
} {
  return {
    implementation: 'sql.js (WASM)',
    isNative: false,
    performance: 'medium',
    requiresBuildTools: false
  };
}
