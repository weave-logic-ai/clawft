import type { VectorBackend } from '../backends/VectorBackend.js';
import type { IDatabaseConnection } from '../types/database.types.js';
export interface AgentDBConfig {
    dbPath?: string;
    namespace?: string;
    enableAttention?: boolean;
    attentionConfig?: Record<string, any>;
    /** Force use of sql.js WASM even if better-sqlite3 is available */
    forceWasm?: boolean;
    /** Vector backend type: 'auto' | 'ruvector' | 'hnswlib' */
    vectorBackend?: 'auto' | 'ruvector' | 'hnswlib';
    /** Vector dimension (default: 384 for MiniLM) */
    vectorDimension?: number;
}
export declare class AgentDB {
    private db;
    private reflexion;
    private skills;
    private causalGraph;
    private embedder;
    vectorBackend: VectorBackend;
    private initialized;
    private config;
    private usingWasm;
    constructor(config?: AgentDBConfig);
    initialize(): Promise<void>;
    /**
     * Initialize database with automatic fallback:
     * 1. Try better-sqlite3 (native, fastest)
     * 2. Fallback to sql.js WASM (no build tools required)
     */
    private initializeDatabase;
    /**
     * Initialize sql.js WASM database
     */
    private initializeSqlJsWasm;
    /**
     * Load database schemas
     */
    private loadSchemas;
    getController(name: string): any;
    close(): Promise<void>;
    get database(): IDatabaseConnection;
    get isWasm(): boolean;
    get vectorBackendName(): string;
}
//# sourceMappingURL=AgentDB.d.ts.map