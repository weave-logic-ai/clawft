/**
 * AgentDBWrapper - Clean API over raw AgentDB v2.0.0-alpha.2.11
 *
 * Provides a unified interface for:
 * - Vector search with HNSW indexing
 * - Memory operations (insert, search, update, delete)
 * - TypeScript-first type safety
 * - Error handling and validation
 *
 * @module agentdb-wrapper
 */
import { AgentDB } from 'agentdb';
/**
 * Main wrapper class for AgentDB operations
 *
 * @example
 * ```typescript
 * const wrapper = new AgentDBWrapper({
 *   dbPath: ':memory:',
 *   dimension: 384,
 *   hnswConfig: { M: 16, efConstruction: 200 }
 * });
 *
 * await wrapper.initialize();
 *
 * // Insert a vector
 * const vector = new Float32Array(384);
 * await wrapper.insert({ vector, metadata: { type: 'test' } });
 *
 * // Search
 * const results = await wrapper.vectorSearch(queryVector, { k: 10 });
 *
 * await wrapper.close();
 * ```
 */
export class AgentDBWrapper {
    agentDB;
    config;
    initialized = false;
    dimension;
    namespace;
    reflexionController;
    embedder;
    vectorBackend;
    // For testing - allow dependency injection
    _agentDB;
    _embedder;
    _vectorBackend;
    /**
     * Create a new AgentDBWrapper instance
     *
     * @param config - Configuration options
     */
    constructor(config = {}) {
        this.config = {
            dbPath: config.dbPath || ':memory:',
            namespace: config.namespace || 'default',
            dimension: config.dimension || 384,
            hnswConfig: {
                M: config.hnswConfig?.M || 16,
                efConstruction: config.hnswConfig?.efConstruction || 200,
                efSearch: config.hnswConfig?.efSearch || 100,
            },
            enableAttention: config.enableAttention || false,
            attentionConfig: config.attentionConfig,
            autoInit: config.autoInit !== false, // Default true
        };
        this.dimension = this.config.dimension;
        this.namespace = this.config.namespace;
        // Auto-initialize if configured
        if (this.config.autoInit) {
            this.initialize().catch((err) => {
                console.error('Auto-initialization failed:', err);
            });
        }
    }
    /**
     * Initialize the AgentDB instance and dependencies
     *
     * @throws {DatabaseError} If initialization fails
     */
    async initialize() {
        if (this.initialized) {
            return;
        }
        try {
            // Use injected dependencies for testing, otherwise create real instances
            if (this._agentDB) {
                this.agentDB = this._agentDB;
                this.embedder = this._embedder;
                this.vectorBackend = this._vectorBackend;
            }
            else {
                // Create real AgentDB instance
                this.agentDB = new AgentDB({
                    dbPath: this.config.dbPath,
                    namespace: this.namespace,
                    enableAttention: this.config.enableAttention,
                    attentionConfig: this.config.attentionConfig,
                });
                await this.agentDB.initialize();
                // Get controllers
                this.reflexionController = this.agentDB.getController('reflexion');
                this.embedder = this.reflexionController?.embedder;
                this.vectorBackend = this.reflexionController?.vectorBackend;
            }
            // Initialize embedder if available
            if (this.embedder && this.embedder.initialize) {
                await this.embedder.initialize();
            }
            this.initialized = true;
        }
        catch (error) {
            throw new Error(`Failed to initialize AgentDB: ${error instanceof Error ? error.message : String(error)}`);
        }
    }
    /**
     * Ensure wrapper is initialized before operations
     *
     * @throws {Error} If not initialized
     */
    ensureInitialized() {
        if (!this.initialized) {
            throw new Error('AgentDBWrapper not initialized. Call initialize() first.');
        }
    }
    /**
     * Validate vector dimension
     *
     * @param vector - Vector to validate
     * @throws {ValidationError} If dimension is invalid
     */
    validateVectorDimension(vector) {
        if (vector.length !== this.dimension) {
            throw new Error(`Invalid vector dimension: expected ${this.dimension}, got ${vector.length}`);
        }
    }
    /**
     * Generate a unique ID
     *
     * @returns A unique identifier
     */
    generateId() {
        return `${Date.now()}-${Math.random().toString(36).substring(2, 11)}`;
    }
    /**
     * Insert a vector with metadata into the database
     *
     * @param options - Insert options
     * @returns The inserted entry with ID
     * @throws {ValidationError} If vector dimension is invalid
     * @throws {DatabaseError} If insertion fails
     */
    async insert(options) {
        this.ensureInitialized();
        this.validateVectorDimension(options.vector);
        const id = options.id || this.generateId();
        const timestamp = Date.now();
        const metadata = {
            ...options.metadata,
            timestamp,
            namespace: options.namespace || this.namespace,
        };
        try {
            const controller = this._agentDB
                ? this._agentDB.getController('reflexion')
                : this.reflexionController;
            await controller.store({
                id,
                vector: options.vector,
                metadata,
            });
            return { id, timestamp };
        }
        catch (error) {
            throw new Error(`Failed to insert vector: ${error instanceof Error ? error.message : String(error)}`);
        }
    }
    /**
     * Search for similar vectors using HNSW indexing
     *
     * @param query - Query vector
     * @param options - Search options
     * @returns Array of search results sorted by similarity (descending)
     * @throws {ValidationError} If query dimension is invalid
     */
    async vectorSearch(query, options = {}) {
        this.ensureInitialized();
        this.validateVectorDimension(query);
        const { k = 10, metric = 'cosine', filter, includeVectors = false, hnswParams, } = options;
        try {
            const controller = this._agentDB
                ? this._agentDB.getController('reflexion')
                : this.reflexionController;
            const results = await controller.retrieve(query, {
                k,
                metric,
                filter,
                includeVectors,
                hnswParams,
            });
            return results.map((result) => ({
                id: result.id,
                score: result.score,
                metadata: result.metadata,
                ...(includeVectors && { vector: result.vector }),
            }));
        }
        catch (error) {
            throw new Error(`Vector search failed: ${error instanceof Error ? error.message : String(error)}`);
        }
    }
    /**
     * Update a vector and/or its metadata
     *
     * @param options - Update options
     * @returns True if update succeeded
     * @throws {ValidationError} If vector dimension is invalid
     * @throws {DatabaseError} If vector not found
     */
    async update(options) {
        this.ensureInitialized();
        if (options.vector) {
            this.validateVectorDimension(options.vector);
        }
        try {
            const controller = this._agentDB
                ? this._agentDB.getController('reflexion')
                : this.reflexionController;
            const updateData = {};
            if (options.vector) {
                updateData.vector = options.vector;
            }
            if (options.metadata) {
                updateData.metadata = options.metadata;
            }
            const result = await controller.update(options.id, updateData);
            if (!result) {
                throw new Error(`Vector not found: ${options.id}`);
            }
            return result;
        }
        catch (error) {
            throw error;
        }
    }
    /**
     * Delete a vector by ID
     *
     * @param options - Delete options
     * @returns True if deletion succeeded, false if not found
     */
    async delete(options) {
        this.ensureInitialized();
        try {
            const controller = this._agentDB
                ? this._agentDB.getController('reflexion')
                : this.reflexionController;
            const result = await controller.delete(options.id);
            return result;
        }
        catch (error) {
            throw new Error(`Delete failed: ${error instanceof Error ? error.message : String(error)}`);
        }
    }
    /**
     * Get a vector by ID
     *
     * @param options - Get options
     * @returns Vector entry or null if not found
     */
    async get(options) {
        this.ensureInitialized();
        try {
            const controller = this._agentDB
                ? this._agentDB.getController('reflexion')
                : this.reflexionController;
            const result = await controller.get(options.id);
            if (!result) {
                return null;
            }
            const entry = {
                id: result.id,
                vector: result.vector,
                metadata: result.metadata,
            };
            // Exclude vector if not requested
            if (options.includeVector === false) {
                delete entry.vector;
            }
            return entry;
        }
        catch (error) {
            throw new Error(`Get failed: ${error instanceof Error ? error.message : String(error)}`);
        }
    }
    /**
     * Insert multiple vectors in a batch
     *
     * @param entries - Array of entries to insert
     * @returns Batch insert result with success/failure counts
     */
    async batchInsert(entries) {
        this.ensureInitialized();
        const startTime = Date.now();
        const result = {
            inserted: 0,
            failed: [],
            duration: 0,
        };
        const controller = this._agentDB
            ? this._agentDB.getController('reflexion')
            : this.reflexionController;
        for (let i = 0; i < entries.length; i++) {
            const entry = entries[i];
            try {
                await this.insert(entry);
                result.inserted++;
            }
            catch (error) {
                result.failed.push({
                    index: i,
                    id: entry.id,
                    error: error instanceof Error ? error.message : String(error),
                });
            }
        }
        result.duration = Date.now() - startTime;
        return result;
    }
    /**
     * Get database statistics
     *
     * @returns Database statistics including vector count and index info
     */
    async getStats() {
        this.ensureInitialized();
        try {
            const backend = this._vectorBackend || this.vectorBackend;
            const stats = await backend.getStats();
            return {
                vectorCount: stats.vectorCount || 0,
                dimension: this.dimension,
                databaseSize: stats.databaseSize || 0,
                hnswStats: stats.hnswStats,
                memoryUsage: stats.memoryUsage,
                indexBuildTime: stats.indexBuildTime,
            };
        }
        catch (error) {
            throw new Error(`Failed to get stats: ${error instanceof Error ? error.message : String(error)}`);
        }
    }
    /**
     * Close the database connection
     */
    async close() {
        if (!this.initialized) {
            return;
        }
        try {
            const db = this._agentDB || this.agentDB;
            await db.close();
            this.initialized = false;
        }
        catch (error) {
            throw new Error(`Failed to close database: ${error instanceof Error ? error.message : String(error)}`);
        }
    }
    /**
     * Get the underlying AgentDB instance (for advanced usage)
     *
     * @returns The raw AgentDB instance
     */
    getRawInstance() {
        this.ensureInitialized();
        return this.agentDB;
    }
}
//# sourceMappingURL=agentdb-wrapper.js.map