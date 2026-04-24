/**
 * AgentDB - Main database wrapper class
 *
 * Provides a unified interface to all AgentDB controllers with:
 * - sql.js WASM for relational storage (with better-sqlite3 fallback)
 * - RuVector for optimized vector search (150x faster than SQLite)
 * - Unified integration passing vector backend to all controllers
 */
import { ReflexionMemory } from '../controllers/ReflexionMemory.js';
import { SkillLibrary } from '../controllers/SkillLibrary.js';
import { CausalMemoryGraph } from '../controllers/CausalMemoryGraph.js';
import { EmbeddingService } from '../controllers/EmbeddingService.js';
import { createBackend } from '../backends/factory.js';
import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';
const __dirname = path.dirname(fileURLToPath(import.meta.url));
export class AgentDB {
    db;
    reflexion;
    skills;
    causalGraph;
    embedder;
    vectorBackend;
    initialized = false;
    config;
    usingWasm = false;
    constructor(config = {}) {
        this.config = config;
    }
    async initialize() {
        if (this.initialized)
            return;
        const dbPath = this.config.dbPath || ':memory:';
        const vectorDimension = this.config.vectorDimension || 384;
        // Initialize database with unified fallback system
        this.db = await this.initializeDatabase(dbPath);
        // Load schemas
        await this.loadSchemas();
        // Initialize embedder with default Xenova model
        this.embedder = new EmbeddingService({
            model: 'Xenova/all-MiniLM-L6-v2',
            dimension: vectorDimension,
            provider: 'transformers'
        });
        await this.embedder.initialize();
        // Initialize vector backend (RuVector preferred, HNSWLib fallback)
        this.vectorBackend = await createBackend(this.config.vectorBackend || 'auto', {
            dimensions: vectorDimension,
            metric: 'cosine'
        });
        // Initialize controllers WITH vector backend for optimized search
        // This enables 150x faster vector search via RuVector instead of SQLite brute-force
        this.reflexion = new ReflexionMemory(this.db, this.embedder, this.vectorBackend);
        this.skills = new SkillLibrary(this.db, this.embedder, this.vectorBackend);
        this.causalGraph = new CausalMemoryGraph(this.db, undefined, // graphBackend - not used in default initialization
        this.embedder, undefined, // config - use defaults
        this.vectorBackend);
        this.initialized = true;
        console.log(`[AgentDB] Initialized with ${this.usingWasm ? 'sql.js WASM' : 'better-sqlite3'} + ${this.vectorBackend.name} vector backend`);
    }
    /**
     * Initialize database with automatic fallback:
     * 1. Try better-sqlite3 (native, fastest)
     * 2. Fallback to sql.js WASM (no build tools required)
     */
    async initializeDatabase(dbPath) {
        // Force WASM if requested
        if (this.config.forceWasm) {
            return this.initializeSqlJsWasm(dbPath);
        }
        // Try better-sqlite3 first (native performance)
        try {
            const Database = (await import('better-sqlite3')).default;
            const db = new Database(dbPath);
            db.pragma('journal_mode = WAL');
            this.usingWasm = false;
            return db;
        }
        catch (error) {
            // better-sqlite3 not available or failed, try sql.js WASM
            console.log('[AgentDB] better-sqlite3 not available, using sql.js WASM');
            return this.initializeSqlJsWasm(dbPath);
        }
    }
    /**
     * Initialize sql.js WASM database
     */
    async initializeSqlJsWasm(dbPath) {
        const { createDatabase } = await import('../db-fallback.js');
        const db = await createDatabase(dbPath);
        this.usingWasm = true;
        return db;
    }
    /**
     * Load database schemas
     */
    async loadSchemas() {
        const schemaPath = path.join(__dirname, '../../schemas/schema.sql');
        if (fs.existsSync(schemaPath)) {
            const schema = fs.readFileSync(schemaPath, 'utf-8');
            this.db.exec(schema);
        }
        const frontierSchemaPath = path.join(__dirname, '../../schemas/frontier-schema.sql');
        if (fs.existsSync(frontierSchemaPath)) {
            const frontierSchema = fs.readFileSync(frontierSchemaPath, 'utf-8');
            this.db.exec(frontierSchema);
        }
    }
    getController(name) {
        if (!this.initialized) {
            throw new Error('AgentDB not initialized. Call initialize() first.');
        }
        switch (name) {
            case 'memory':
            case 'reflexion':
                return this.reflexion;
            case 'skills':
                return this.skills;
            case 'causal':
            case 'causalGraph':
                return this.causalGraph;
            default:
                throw new Error(`Unknown controller: ${name}`);
        }
    }
    async close() {
        if (this.db) {
            this.db.close();
        }
    }
    // Expose database for advanced usage
    get database() {
        return this.db;
    }
    // Check if using WASM backend
    get isWasm() {
        return this.usingWasm;
    }
    // Get vector backend info
    get vectorBackendName() {
        return this.vectorBackend?.name || 'none';
    }
}
//# sourceMappingURL=AgentDB.js.map