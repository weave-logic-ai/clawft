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

import Database from 'better-sqlite3';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

interface MigrationResult {
  success: boolean;
  migration: string;
  timeMs: number;
  error?: string;
}

/**
 * Apply a migration to a database
 */
function applyMigration(dbPath: string, migrationFile: string): MigrationResult {
  const startTime = Date.now();

  try {
    // Validate database path
    if (!fs.existsSync(dbPath)) {
      throw new Error(`Database not found: ${dbPath}`);
    }

    // Validate migration file
    const migrationPath = path.resolve(__dirname, migrationFile);
    if (!fs.existsSync(migrationPath)) {
      throw new Error(`Migration file not found: ${migrationPath}`);
    }

    console.log(`\n📦 AgentDB Migration Runner`);
    console.log(`   Database: ${dbPath}`);
    console.log(`   Migration: ${migrationFile}\n`);

    // Open database
    const db = new Database(dbPath);

    // Enable foreign keys
    db.pragma('foreign_keys = ON');

    // Read migration SQL
    const migrationSQL = fs.readFileSync(migrationPath, 'utf-8');

    // Count statements (rough estimate)
    const statementCount = migrationSQL.split(';').filter((s) => s.trim()).length;
    console.log(`   Statements: ~${statementCount}`);

    // Apply migration in transaction
    console.log(`\n⏳ Applying migration...`);

    db.exec('BEGIN TRANSACTION');

    try {
      db.exec(migrationSQL);
      db.exec('COMMIT');

      const timeMs = Date.now() - startTime;

      console.log(`\n✅ Migration applied successfully`);
      console.log(`   Time: ${timeMs}ms\n`);

      // Verify indexes
      console.log(`📊 Verifying indexes...`);
      const indexes = db
        .prepare(
          `
        SELECT name, tbl_name
        FROM sqlite_master
        WHERE type = 'index' AND name LIKE 'idx_%'
        ORDER BY tbl_name, name
      `
        )
        .all();

      console.log(`   Total indexes: ${indexes.length}`);

      // Group by table
      const indexesByTable = indexes.reduce((acc: Record<string, string[]>, idx: any) => {
        if (!acc[idx.tbl_name]) acc[idx.tbl_name] = [];
        acc[idx.tbl_name].push(idx.name);
        return acc;
      }, {});

      Object.entries(indexesByTable).forEach(([table, idxs]) => {
        console.log(`   ${table}: ${idxs.length} indexes`);
      });

      // Analyze database
      console.log(`\n🔍 Analyzing database...`);
      db.exec('ANALYZE');

      // Get database stats
      const stats = db.pragma('page_count');
      const pageSize = db.pragma('page_size');
      const dbSize = (stats as any)[0].page_count * (pageSize as any)[0].page_size;

      console.log(`   Database size: ${(dbSize / 1024 / 1024).toFixed(2)} MB`);
      console.log(`   Pages: ${(stats as any)[0].page_count}`);

      db.close();

      return {
        success: true,
        migration: migrationFile,
        timeMs,
      };
    } catch (error) {
      db.exec('ROLLBACK');
      throw error;
    }
  } catch (error) {
    const timeMs = Date.now() - startTime;
    const errorMsg = error instanceof Error ? error.message : String(error);

    console.error(`\n❌ Migration failed: ${errorMsg}\n`);

    return {
      success: false,
      migration: migrationFile,
      timeMs,
      error: errorMsg,
    };
  }
}

/**
 * Main CLI entry point
 */
function main() {
  const args = process.argv.slice(2);

  if (args.length < 2) {
    console.error(`
Usage: node apply-migration.ts <db-path> <migration-file>

Examples:
  node apply-migration.ts ./memory.db 003_composite_indexes.sql
  node apply-migration.ts /path/to/agentdb.db 003_composite_indexes.sql

Available migrations:
  - 003_composite_indexes.sql (Performance optimization)
`);
    process.exit(1);
  }

  const [dbPath, migrationFile] = args;
  const result = applyMigration(dbPath, migrationFile);

  process.exit(result.success ? 0 : 1);
}

// Run if called directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}

export { applyMigration };
