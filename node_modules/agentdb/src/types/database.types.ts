/**
 * Database Type Definitions for AgentDB
 *
 * Provides strong typing for database operations, supporting both
 * better-sqlite3 and sql.js backends with comprehensive type safety.
 *
 * Key Features:
 * - Generic query result typing
 * - Type-safe prepared statements
 * - Null safety for database operations
 * - Support for both sync (better-sqlite3) and async (sql.js) patterns
 */

/**
 * Prepared statement interface for type-safe queries
 */
export interface IPreparedStatement<T = any> {
  /**
   * Execute a query and get a single row result
   * @param params - Query parameters
   * @returns Single row or undefined if no match
   */
  get(...params: any[]): T | undefined;

  /**
   * Execute a query and get all matching rows
   * @param params - Query parameters
   * @returns Array of matching rows
   */
  all(...params: any[]): T[];

  /**
   * Execute a query that modifies data (INSERT, UPDATE, DELETE)
   * @param params - Query parameters
   * @returns Result with metadata about the operation
   */
  run(...params: any[]): IRunResult;

  /**
   * Finalize the statement and free resources
   */
  finalize?(): void;
}

/**
 * Result of a data modification query (INSERT, UPDATE, DELETE)
 */
export interface IRunResult {
  /**
   * Number of rows affected by the query
   */
  changes: number;

  /**
   * Last inserted row ID (for INSERT operations)
   * Can be string (sql.js), number, or bigint (better-sqlite3)
   */
  lastInsertRowid?: number | bigint | string;
}

/**
 * Database connection interface
 *
 * Abstracts over both better-sqlite3 and sql.js backends
 * Provides type-safe methods for all database operations
 */
export interface IDatabaseConnection {
  /**
   * Execute one or more SQL statements without returning results
   * Useful for schema creation and batch operations
   *
   * @param sql - SQL statement(s) to execute
   * @example
   * db.exec(`
   *   CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT);
   *   CREATE INDEX idx_name ON users(name);
   * `);
   */
  exec(sql: string): void;

  /**
   * Prepare a SQL statement for execution
   *
   * @param sql - SQL statement with optional placeholders (?)
   * @returns Prepared statement for type-safe execution
   * @example
   * const stmt = db.prepare('SELECT * FROM users WHERE id = ?');
   * const user = stmt.get(123);
   */
  prepare<T = any>(sql: string): IPreparedStatement<T>;

  /**
   * Execute a SQL query and return all results
   * Convenience method for one-off queries
   *
   * @param sql - SQL query
   * @param params - Query parameters
   * @returns Array of result rows
   * @deprecated Use prepare().all() for better performance and type safety
   */
  all?<T = any>(sql: string, ...params: any[]): T[];

  /**
   * Execute a SQL query and return a single result
   * Convenience method for one-off queries
   *
   * @param sql - SQL query
   * @param params - Query parameters
   * @returns Single row or undefined
   * @deprecated Use prepare().get() for better performance and type safety
   */
  get?<T = any>(sql: string, ...params: any[]): T | undefined;

  /**
   * Execute a data modification query
   * Convenience method for one-off queries
   *
   * @param sql - SQL statement (INSERT, UPDATE, DELETE)
   * @param params - Query parameters
   * @returns Run result with metadata
   * @deprecated Use prepare().run() for better performance and type safety
   */
  run?(sql: string, ...params: any[]): IRunResult;

  /**
   * Close the database connection and free resources
   */
  close(): void;

  /**
   * Transaction support (better-sqlite3 style)
   * Creates a transaction wrapper function
   * Note: Exact signature depends on database backend
   */
  transaction?: any;

  /**
   * Check if database is in-memory (sql.js)
   */
  inMemory?: boolean;

  /**
   * Export database to buffer (sql.js only)
   */
  export?(): Uint8Array;

  /**
   * Get database file path (better-sqlite3 only)
   */
  readonly name?: string;

  /**
   * Check if database is open
   */
  readonly open?: boolean;

  /**
   * Enable WAL mode (better-sqlite3 only)
   */
  pragma?(pragma: string, value?: any): any;
}

/**
 * Generic query result type helper
 *
 * Use with prepare<T>() for type-safe query results
 * @example
 * interface User { id: number; name: string; }
 * const stmt = db.prepare<User>('SELECT * FROM users WHERE id = ?');
 * const user: User | undefined = stmt.get(123);
 */
export type QueryResult<T> = T;

/**
 * Type guard to check if a value is a valid database connection
 */
export function isDatabaseConnection(db: any): db is IDatabaseConnection {
  return (
    db &&
    typeof db === 'object' &&
    typeof db.exec === 'function' &&
    typeof db.prepare === 'function' &&
    typeof db.close === 'function'
  );
}

/**
 * Type guard to check if a value is a prepared statement
 */
export function isPreparedStatement(stmt: any): stmt is IPreparedStatement {
  return (
    stmt &&
    typeof stmt === 'object' &&
    typeof stmt.get === 'function' &&
    typeof stmt.all === 'function' &&
    typeof stmt.run === 'function'
  );
}

/**
 * Helper to normalize lastInsertRowid to number
 * Handles differences between better-sqlite3 (number/bigint) and sql.js (string)
 */
export function normalizeRowId(rowid: number | bigint | string | undefined): number {
  if (rowid === undefined) {
    throw new Error('lastInsertRowid is undefined');
  }
  if (typeof rowid === 'string') {
    return parseInt(rowid, 10);
  }
  if (typeof rowid === 'bigint') {
    return Number(rowid);
  }
  return rowid;
}

/**
 * Helper to create a type-safe prepared statement wrapper
 */
export function createTypedStatement<T>(
  db: IDatabaseConnection,
  sql: string
): IPreparedStatement<T> {
  return db.prepare<T>(sql);
}

/**
 * Row type helpers for common database tables
 */
export namespace DatabaseRows {
  /**
   * Episode row from episodes table
   */
  export interface Episode {
    id: number;
    ts: number;
    session_id: string;
    task: string;
    input: string | null;
    output: string | null;
    critique: string | null;
    reward: number;
    success: number; // SQLite stores boolean as 0/1
    latency_ms: number | null;
    tokens_used: number | null;
    tags: string | null; // JSON string
    metadata: string | null; // JSON string
  }

  /**
   * Pattern row from reasoning_patterns table
   */
  export interface ReasoningPattern {
    id: number;
    ts: number;
    task_type: string;
    approach: string;
    success_rate: number;
    uses: number;
    avg_reward: number;
    tags: string | null; // JSON string
    metadata: string | null; // JSON string
  }

  /**
   * Skill row from skills table
   */
  export interface Skill {
    id: number;
    name: string;
    description: string | null;
    signature: string; // JSON string
    code: string | null;
    success_rate: number;
    uses: number;
    avg_reward: number;
    avg_latency_ms: number;
    created_from_episode: number | null;
    created_at: number;
    metadata: string | null; // JSON string
  }

  /**
   * Causal edge row from causal_edges table
   */
  export interface CausalEdge {
    id: number;
    from_memory_id: number;
    from_memory_type: string;
    to_memory_id: number;
    to_memory_type: string;
    similarity: number;
    uplift: number | null;
    confidence: number;
    sample_size: number | null;
    evidence_ids: string | null; // JSON string
    confounder_score: number | null;
    mechanism: string | null;
    metadata: string | null; // JSON string
  }

  /**
   * Count result for aggregate queries
   */
  export interface CountResult {
    count: number;
  }

  /**
   * Average result for aggregate queries
   */
  export interface AverageResult {
    avg_success_rate: number | null;
    avg_uses: number | null;
    avg_reward: number | null;
    avg_latency: number | null;
  }
}

/**
 * Database factory for creating type-safe connections
 */
export interface IDatabaseFactory {
  /**
   * Create a new database connection
   * @param path - Database file path or ':memory:' for in-memory
   * @returns Database connection
   */
  create(path: string): IDatabaseConnection;

  /**
   * Open an existing database
   * @param path - Database file path
   * @returns Database connection
   */
  open(path: string): IDatabaseConnection;
}
