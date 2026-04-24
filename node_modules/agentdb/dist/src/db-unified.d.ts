/**
 * Unified Database Layer for AgentDB v2
 *
 * Architecture:
 * - PRIMARY: RuVector GraphDatabase (@ruvector/graph-node) for new databases
 * - FALLBACK: SQLite (sql.js) for legacy databases
 *
 * Detection Logic:
 * 1. Check if database file exists
 * 2. If exists, check file signature to determine type
 * 3. If new database or .graph extension → use GraphDatabase
 * 4. If .db extension and SQLite signature → use SQLite (legacy mode)
 *
 * Migration:
 * - Provides migration tool to convert SQLite → GraphDatabase
 * - Maintains backward compatibility with existing databases
 */
import { GraphDatabaseAdapter } from './backends/graph/GraphDatabaseAdapter.js';
export type DatabaseMode = 'graph' | 'sqlite-legacy';
export interface UnifiedDatabaseConfig {
    path: string;
    dimensions?: number;
    forceMode?: DatabaseMode;
    autoMigrate?: boolean;
}
/**
 * Unified Database - Smart detection and mode selection
 */
export declare class UnifiedDatabase {
    private mode;
    private graphDb?;
    private sqliteDb?;
    private config;
    constructor(config: UnifiedDatabaseConfig);
    /**
     * Initialize database with automatic mode detection
     */
    initialize(embedder: any): Promise<void>;
    /**
     * Initialize the selected database mode
     */
    private initializeMode;
    /**
     * Check if file is a SQLite database
     */
    private isSQLiteDatabase;
    /**
     * Migrate SQLite database to RuVector GraphDatabase
     */
    private migrateSQLiteToGraph;
    /**
     * Get the active database mode
     */
    getMode(): DatabaseMode;
    /**
     * Get the graph database (if in graph mode)
     */
    getGraphDatabase(): GraphDatabaseAdapter | undefined;
    /**
     * Get the SQLite database (if in legacy mode)
     */
    getSQLiteDatabase(): any | undefined;
    /**
     * Execute a query (auto-routes to correct database)
     */
    query(queryOrCypher: string): Promise<any>;
    /**
     * Close database
     */
    close(): void;
}
/**
 * Create unified database (smart mode detection)
 */
export declare function createUnifiedDatabase(path: string, embedder: any, options?: Partial<UnifiedDatabaseConfig>): Promise<UnifiedDatabase>;
//# sourceMappingURL=db-unified.d.ts.map