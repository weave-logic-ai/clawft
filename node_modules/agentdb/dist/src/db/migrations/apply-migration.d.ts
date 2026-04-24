#!/usr/bin/env node
/**
 * AgentDB Migration Runner
 *
 * Applies database migrations to an existing AgentDB database.
 *
 * Usage:
 *   node apply-migration.ts <db-path> <migration-file>
 *   node apply-migration.ts ./memory.db 003_composite_indexes.sql
 */
interface MigrationResult {
    success: boolean;
    migration: string;
    timeMs: number;
    error?: string;
}
/**
 * Apply a migration to a database
 */
declare function applyMigration(dbPath: string, migrationFile: string): MigrationResult;
export { applyMigration };
//# sourceMappingURL=apply-migration.d.ts.map