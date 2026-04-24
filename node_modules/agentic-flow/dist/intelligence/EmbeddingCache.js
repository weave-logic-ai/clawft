/**
 * EmbeddingCache - Persistent cache for embeddings
 *
 * Makes ONNX embeddings practical by caching across sessions:
 * - First embed: ~400ms (ONNX inference)
 * - Cached embed: ~0.1ms (SQLite lookup) or ~0.01ms (in-memory fallback)
 *
 * Storage: ~/.agentic-flow/embedding-cache.db (if SQLite available)
 *
 * Windows Compatibility:
 * - Falls back to in-memory cache if better-sqlite3 compilation fails
 * - No native module compilation required for basic functionality
 */
import { existsSync, mkdirSync, statSync, readFileSync, writeFileSync } from 'fs';
import { join } from 'path';
import { homedir } from 'os';
import { createHash } from 'crypto';
// Default config
const DEFAULT_CONFIG = {
    maxEntries: 10000,
    maxAgeDays: 30,
    dbPath: join(homedir(), '.agentic-flow', 'embedding-cache.db'),
    dimension: 384,
    forceMemory: false,
};
// Check if better-sqlite3 is available (native, fastest)
let BetterSqlite3 = null;
let nativeSqliteAvailable = false;
// Check if sql.js is available (WASM, cross-platform)
let SqlJs = null;
let wasmSqliteAvailable = false;
try {
    // Try native SQLite first (fastest)
    BetterSqlite3 = require('better-sqlite3');
    nativeSqliteAvailable = true;
}
catch {
    // Native not available, try WASM fallback
    try {
        SqlJs = require('sql.js');
        wasmSqliteAvailable = true;
    }
    catch {
        // Neither available, will use memory cache
    }
}
const sqliteAvailable = nativeSqliteAvailable || wasmSqliteAvailable;
/**
 * In-memory cache fallback for Windows compatibility
 */
class MemoryCache {
    cache = new Map();
    maxEntries;
    hits = 0;
    misses = 0;
    constructor(maxEntries = 10000) {
        this.maxEntries = maxEntries;
    }
    get(hash) {
        const entry = this.cache.get(hash);
        if (entry) {
            entry.hits++;
            entry.accessed = Date.now();
            this.hits++;
            return { embedding: entry.embedding, dimension: entry.embedding.length };
        }
        this.misses++;
        return null;
    }
    set(hash, text, embedding, model) {
        const now = Date.now();
        this.cache.set(hash, {
            embedding,
            model,
            hits: 1,
            created: now,
            accessed: now,
        });
        // Evict if over limit
        if (this.cache.size > this.maxEntries) {
            this.evictLRU(Math.ceil(this.maxEntries * 0.1));
        }
    }
    has(hash) {
        return this.cache.has(hash);
    }
    evictLRU(count) {
        const entries = Array.from(this.cache.entries())
            .sort((a, b) => a[1].accessed - b[1].accessed)
            .slice(0, count);
        for (const [key] of entries) {
            this.cache.delete(key);
        }
    }
    clear() {
        this.cache.clear();
        this.hits = 0;
        this.misses = 0;
    }
    getStats() {
        const entries = Array.from(this.cache.values());
        const oldest = entries.length > 0 ? Math.min(...entries.map(e => e.created)) : 0;
        const newest = entries.length > 0 ? Math.max(...entries.map(e => e.created)) : 0;
        return {
            totalEntries: this.cache.size,
            hits: this.hits,
            misses: this.misses,
            hitRate: this.hits + this.misses > 0 ? this.hits / (this.hits + this.misses) : 0,
            dbSizeBytes: this.cache.size * 384 * 4, // Approximate
            oldestEntry: oldest,
            newestEntry: newest,
            backend: 'memory',
        };
    }
}
/**
 * WASM SQLite cache (sql.js) - Cross-platform with persistence
 * Works on Windows without native compilation
 */
class WasmSqliteCache {
    db = null;
    config;
    hits = 0;
    misses = 0;
    dirty = false;
    saveTimeout = null;
    constructor(config) {
        this.config = config;
    }
    async init() {
        if (this.db)
            return;
        // Ensure directory exists
        const dir = join(homedir(), '.agentic-flow');
        if (!existsSync(dir)) {
            mkdirSync(dir, { recursive: true });
        }
        // Initialize sql.js
        const SQL = await SqlJs();
        // Load existing database or create new
        const dbPath = this.config.dbPath.replace('.db', '-wasm.db');
        try {
            if (existsSync(dbPath)) {
                const buffer = readFileSync(dbPath);
                this.db = new SQL.Database(buffer);
            }
            else {
                this.db = new SQL.Database();
            }
        }
        catch {
            this.db = new SQL.Database();
        }
        this.initSchema();
        this.cleanupOldEntries();
    }
    initSchema() {
        this.db.run(`
      CREATE TABLE IF NOT EXISTS embeddings (
        hash TEXT PRIMARY KEY,
        text TEXT NOT NULL,
        embedding BLOB NOT NULL,
        dimension INTEGER NOT NULL,
        model TEXT NOT NULL,
        hits INTEGER DEFAULT 1,
        created_at INTEGER NOT NULL,
        last_accessed INTEGER NOT NULL
      )
    `);
        this.db.run(`CREATE INDEX IF NOT EXISTS idx_last_accessed ON embeddings(last_accessed)`);
        this.db.run(`CREATE INDEX IF NOT EXISTS idx_created_at ON embeddings(created_at)`);
    }
    save() {
        // Debounce saves
        if (this.saveTimeout)
            return;
        this.saveTimeout = setTimeout(() => {
            try {
                const data = this.db.export();
                const buffer = Buffer.from(data);
                const dbPath = this.config.dbPath.replace('.db', '-wasm.db');
                writeFileSync(dbPath, buffer);
                this.dirty = false;
            }
            catch (err) {
                console.warn('[WasmSqliteCache] Save failed:', err);
            }
            this.saveTimeout = null;
        }, 1000);
    }
    get(hash) {
        if (!this.db)
            return null;
        const stmt = this.db.prepare(`SELECT embedding, dimension FROM embeddings WHERE hash = ?`);
        stmt.bind([hash]);
        if (stmt.step()) {
            const row = stmt.getAsObject();
            stmt.free();
            this.hits++;
            this.db.run(`UPDATE embeddings SET hits = hits + 1, last_accessed = ? WHERE hash = ?`, [Date.now(), hash]);
            this.dirty = true;
            this.save();
            // Convert Uint8Array to Float32Array
            const uint8 = row.embedding;
            const float32 = new Float32Array(uint8.buffer, uint8.byteOffset, row.dimension);
            return { embedding: float32, dimension: row.dimension };
        }
        stmt.free();
        this.misses++;
        return null;
    }
    set(hash, text, embedding, model) {
        if (!this.db)
            return;
        const now = Date.now();
        const buffer = new Uint8Array(embedding.buffer, embedding.byteOffset, embedding.byteLength);
        this.db.run(`INSERT OR REPLACE INTO embeddings (hash, text, embedding, dimension, model, hits, created_at, last_accessed)
       VALUES (?, ?, ?, ?, ?, 1, ?, ?)`, [hash, text, buffer, embedding.length, model, now, now]);
        this.dirty = true;
        this.maybeEvict();
        this.save();
    }
    has(hash) {
        if (!this.db)
            return false;
        const stmt = this.db.prepare(`SELECT 1 FROM embeddings WHERE hash = ? LIMIT 1`);
        stmt.bind([hash]);
        const found = stmt.step();
        stmt.free();
        return found;
    }
    maybeEvict() {
        const countStmt = this.db.prepare(`SELECT COUNT(*) as count FROM embeddings`);
        countStmt.step();
        const count = countStmt.getAsObject().count;
        countStmt.free();
        if (count > this.config.maxEntries) {
            const toEvict = Math.ceil(this.config.maxEntries * 0.1);
            this.db.run(`DELETE FROM embeddings WHERE hash IN (
        SELECT hash FROM embeddings ORDER BY last_accessed ASC LIMIT ?
      )`, [toEvict]);
        }
    }
    cleanupOldEntries() {
        const cutoff = Date.now() - (this.config.maxAgeDays * 24 * 60 * 60 * 1000);
        this.db.run(`DELETE FROM embeddings WHERE created_at < ?`, [cutoff]);
    }
    clear() {
        if (this.db) {
            this.db.run('DELETE FROM embeddings');
            this.hits = 0;
            this.misses = 0;
            this.save();
        }
    }
    close() {
        if (this.saveTimeout) {
            clearTimeout(this.saveTimeout);
            // Force save
            try {
                const data = this.db.export();
                const buffer = Buffer.from(data);
                const dbPath = this.config.dbPath.replace('.db', '-wasm.db');
                writeFileSync(dbPath, buffer);
            }
            catch { }
        }
        if (this.db) {
            this.db.close();
            this.db = null;
        }
    }
    getStats() {
        if (!this.db) {
            return {
                totalEntries: 0,
                hits: this.hits,
                misses: this.misses,
                hitRate: 0,
                dbSizeBytes: 0,
                oldestEntry: 0,
                newestEntry: 0,
                backend: 'memory',
            };
        }
        const countStmt = this.db.prepare(`SELECT COUNT(*) as count FROM embeddings`);
        countStmt.step();
        const count = countStmt.getAsObject().count;
        countStmt.free();
        const oldestStmt = this.db.prepare(`SELECT MIN(created_at) as oldest FROM embeddings`);
        oldestStmt.step();
        const oldest = oldestStmt.getAsObject().oldest || 0;
        oldestStmt.free();
        const newestStmt = this.db.prepare(`SELECT MAX(created_at) as newest FROM embeddings`);
        newestStmt.step();
        const newest = newestStmt.getAsObject().newest || 0;
        newestStmt.free();
        let dbSizeBytes = 0;
        try {
            const dbPath = this.config.dbPath.replace('.db', '-wasm.db');
            const stats = statSync(dbPath);
            dbSizeBytes = stats.size;
        }
        catch { }
        return {
            totalEntries: count,
            hits: this.hits,
            misses: this.misses,
            hitRate: this.hits + this.misses > 0 ? this.hits / (this.hits + this.misses) : 0,
            dbSizeBytes,
            oldestEntry: oldest,
            newestEntry: newest,
            backend: 'file',
        };
    }
}
/**
 * Native SQLite cache (better-sqlite3) - Fastest option
 */
class SqliteCache {
    db;
    config;
    hits = 0;
    misses = 0;
    // Prepared statements for performance
    stmtGet;
    stmtInsert;
    stmtUpdateHits;
    stmtCount;
    stmtEvictOld;
    stmtEvictLRU;
    stmtHas;
    constructor(config) {
        this.config = config;
        // Ensure directory exists
        const dir = join(homedir(), '.agentic-flow');
        if (!existsSync(dir)) {
            mkdirSync(dir, { recursive: true });
        }
        // Open database with WAL mode for better concurrency
        this.db = new BetterSqlite3(this.config.dbPath);
        this.db.pragma('journal_mode = WAL');
        this.db.pragma('synchronous = NORMAL');
        this.db.pragma('cache_size = 10000');
        this.initSchema();
        this.prepareStatements();
        this.cleanupOldEntries();
    }
    initSchema() {
        this.db.exec(`
      CREATE TABLE IF NOT EXISTS embeddings (
        hash TEXT PRIMARY KEY,
        text TEXT NOT NULL,
        embedding BLOB NOT NULL,
        dimension INTEGER NOT NULL,
        model TEXT NOT NULL,
        hits INTEGER DEFAULT 1,
        created_at INTEGER NOT NULL,
        last_accessed INTEGER NOT NULL
      );

      CREATE INDEX IF NOT EXISTS idx_last_accessed ON embeddings(last_accessed);
      CREATE INDEX IF NOT EXISTS idx_created_at ON embeddings(created_at);
      CREATE INDEX IF NOT EXISTS idx_model ON embeddings(model);
    `);
    }
    prepareStatements() {
        this.stmtGet = this.db.prepare(`
      SELECT embedding, dimension FROM embeddings WHERE hash = ?
    `);
        this.stmtInsert = this.db.prepare(`
      INSERT OR REPLACE INTO embeddings (hash, text, embedding, dimension, model, hits, created_at, last_accessed)
      VALUES (?, ?, ?, ?, ?, 1, ?, ?)
    `);
        this.stmtUpdateHits = this.db.prepare(`
      UPDATE embeddings SET hits = hits + 1, last_accessed = ? WHERE hash = ?
    `);
        this.stmtCount = this.db.prepare(`SELECT COUNT(*) as count FROM embeddings`);
        this.stmtEvictOld = this.db.prepare(`
      DELETE FROM embeddings WHERE created_at < ?
    `);
        this.stmtEvictLRU = this.db.prepare(`
      DELETE FROM embeddings WHERE hash IN (
        SELECT hash FROM embeddings ORDER BY last_accessed ASC LIMIT ?
      )
    `);
        this.stmtHas = this.db.prepare(`SELECT 1 FROM embeddings WHERE hash = ? LIMIT 1`);
    }
    get(hash) {
        const row = this.stmtGet.get(hash);
        if (row) {
            this.hits++;
            this.stmtUpdateHits.run(Date.now(), hash);
            return {
                embedding: new Float32Array(row.embedding.buffer, row.embedding.byteOffset, row.dimension),
                dimension: row.dimension,
            };
        }
        this.misses++;
        return null;
    }
    set(hash, text, embedding, model) {
        const now = Date.now();
        const buffer = Buffer.from(embedding.buffer, embedding.byteOffset, embedding.byteLength);
        this.stmtInsert.run(hash, text, buffer, embedding.length, model, now, now);
        this.maybeEvict();
    }
    has(hash) {
        return this.stmtHas.get(hash) !== undefined;
    }
    maybeEvict() {
        const count = this.stmtCount.get().count;
        if (count > this.config.maxEntries) {
            const toEvict = Math.ceil(this.config.maxEntries * 0.1);
            this.stmtEvictLRU.run(toEvict);
        }
    }
    cleanupOldEntries() {
        const cutoff = Date.now() - (this.config.maxAgeDays * 24 * 60 * 60 * 1000);
        this.stmtEvictOld.run(cutoff);
    }
    clear() {
        this.db.exec('DELETE FROM embeddings');
        this.hits = 0;
        this.misses = 0;
    }
    vacuum() {
        this.db.exec('VACUUM');
    }
    close() {
        this.db.close();
    }
    getStats() {
        const count = this.stmtCount.get().count;
        const oldest = this.db.prepare(`SELECT MIN(created_at) as oldest FROM embeddings`).get();
        const newest = this.db.prepare(`SELECT MAX(created_at) as newest FROM embeddings`).get();
        let dbSizeBytes = 0;
        try {
            const stats = statSync(this.config.dbPath);
            dbSizeBytes = stats.size;
        }
        catch { }
        return {
            totalEntries: count,
            hits: this.hits,
            misses: this.misses,
            hitRate: this.hits + this.misses > 0 ? this.hits / (this.hits + this.misses) : 0,
            dbSizeBytes,
            oldestEntry: oldest.oldest || 0,
            newestEntry: newest.newest || 0,
            backend: 'sqlite',
        };
    }
}
/**
 * EmbeddingCache - Auto-selects best available backend
 *
 * Backend priority:
 * 1. Native SQLite (better-sqlite3) - Fastest, 9000x speedup
 * 2. WASM SQLite (sql.js) - Cross-platform with persistence
 * 3. Memory cache - Fallback, no persistence
 */
export class EmbeddingCache {
    backend;
    config;
    wasmInitPromise = null;
    constructor(config = {}) {
        this.config = { ...DEFAULT_CONFIG, ...config };
        // Try native SQLite first (fastest)
        if (nativeSqliteAvailable && !this.config.forceMemory) {
            try {
                this.backend = new SqliteCache(this.config);
                return;
            }
            catch (err) {
                console.warn('[EmbeddingCache] Native SQLite failed, trying WASM fallback');
            }
        }
        // Try WASM SQLite second (cross-platform with persistence)
        if (wasmSqliteAvailable && !this.config.forceMemory) {
            this.backend = new WasmSqliteCache(this.config);
            this.wasmInitPromise = this.backend.init().catch(err => {
                console.warn('[EmbeddingCache] WASM SQLite init failed, using memory cache');
                this.backend = new MemoryCache(this.config.maxEntries);
            });
            return;
        }
        // Fallback to memory cache
        this.backend = new MemoryCache(this.config.maxEntries);
    }
    /**
     * Ensure WASM backend is initialized (if using)
     */
    async ensureInit() {
        if (this.wasmInitPromise) {
            await this.wasmInitPromise;
        }
    }
    /**
     * Generate hash key for text + model combination
     */
    hashKey(text, model = 'default') {
        return createHash('sha256').update(`${model}:${text}`).digest('hex').slice(0, 32);
    }
    /**
     * Get embedding from cache
     */
    get(text, model = 'default') {
        const hash = this.hashKey(text, model);
        const result = this.backend.get(hash);
        return result ? result.embedding : null;
    }
    /**
     * Store embedding in cache
     */
    set(text, embedding, model = 'default') {
        const hash = this.hashKey(text, model);
        if (this.backend instanceof SqliteCache) {
            this.backend.set(hash, text, embedding, model);
        }
        else {
            this.backend.set(hash, text, embedding, model);
        }
    }
    /**
     * Check if text is cached
     */
    has(text, model = 'default') {
        const hash = this.hashKey(text, model);
        return this.backend.has(hash);
    }
    /**
     * Get multiple embeddings at once
     */
    getMany(texts, model = 'default') {
        const result = new Map();
        for (const text of texts) {
            const embedding = this.get(text, model);
            if (embedding) {
                result.set(text, embedding);
            }
        }
        return result;
    }
    /**
     * Store multiple embeddings at once
     */
    setMany(entries, model = 'default') {
        for (const { text, embedding } of entries) {
            this.set(text, embedding, model);
        }
    }
    /**
     * Get cache statistics
     */
    getStats() {
        return this.backend.getStats();
    }
    /**
     * Clear all cached embeddings
     */
    clear() {
        this.backend.clear();
    }
    /**
     * Vacuum database (SQLite only)
     */
    vacuum() {
        if (this.backend instanceof SqliteCache) {
            this.backend.vacuum();
        }
    }
    /**
     * Close database connection
     */
    close() {
        if (this.backend instanceof SqliteCache || this.backend instanceof WasmSqliteCache) {
            this.backend.close();
        }
    }
    /**
     * Check if using SQLite backend (native or WASM)
     */
    isSqliteBackend() {
        return this.backend instanceof SqliteCache || this.backend instanceof WasmSqliteCache;
    }
    /**
     * Get backend type
     */
    getBackendType() {
        if (this.backend instanceof SqliteCache)
            return 'native';
        if (this.backend instanceof WasmSqliteCache)
            return 'wasm';
        return 'memory';
    }
}
// Singleton instance
let cacheInstance = null;
/**
 * Get the singleton embedding cache
 */
export function getEmbeddingCache(config) {
    if (!cacheInstance) {
        cacheInstance = new EmbeddingCache(config);
    }
    return cacheInstance;
}
/**
 * Reset the cache singleton (for testing)
 */
export function resetEmbeddingCache() {
    if (cacheInstance) {
        cacheInstance.close();
        cacheInstance = null;
    }
}
/**
 * Check if SQLite is available
 */
export function isSqliteAvailable() {
    return sqliteAvailable;
}
//# sourceMappingURL=EmbeddingCache.js.map