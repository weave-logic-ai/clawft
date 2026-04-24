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
 * Type guard to check if a value is a valid database connection
 */
export function isDatabaseConnection(db) {
    return (db &&
        typeof db === 'object' &&
        typeof db.exec === 'function' &&
        typeof db.prepare === 'function' &&
        typeof db.close === 'function');
}
/**
 * Type guard to check if a value is a prepared statement
 */
export function isPreparedStatement(stmt) {
    return (stmt &&
        typeof stmt === 'object' &&
        typeof stmt.get === 'function' &&
        typeof stmt.all === 'function' &&
        typeof stmt.run === 'function');
}
/**
 * Helper to normalize lastInsertRowid to number
 * Handles differences between better-sqlite3 (number/bigint) and sql.js (string)
 */
export function normalizeRowId(rowid) {
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
export function createTypedStatement(db, sql) {
    return db.prepare(sql);
}
//# sourceMappingURL=database.types.js.map