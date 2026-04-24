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
import type { AgentDBConfig, VectorEntry, VectorSearchOptions, VectorSearchResult, MemoryInsertOptions, MemoryUpdateOptions, MemoryDeleteOptions, MemoryGetOptions, AgentDBStats, BatchInsertResult } from '../types/agentdb.js';
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
export declare class AgentDBWrapper {
    private agentDB;
    private config;
    private initialized;
    private dimension;
    private namespace;
    private reflexionController;
    private embedder;
    private vectorBackend;
    _agentDB?: any;
    _embedder?: any;
    _vectorBackend?: any;
    /**
     * Create a new AgentDBWrapper instance
     *
     * @param config - Configuration options
     */
    constructor(config?: AgentDBConfig);
    /**
     * Initialize the AgentDB instance and dependencies
     *
     * @throws {DatabaseError} If initialization fails
     */
    initialize(): Promise<void>;
    /**
     * Ensure wrapper is initialized before operations
     *
     * @throws {Error} If not initialized
     */
    private ensureInitialized;
    /**
     * Validate vector dimension
     *
     * @param vector - Vector to validate
     * @throws {ValidationError} If dimension is invalid
     */
    private validateVectorDimension;
    /**
     * Generate a unique ID
     *
     * @returns A unique identifier
     */
    private generateId;
    /**
     * Insert a vector with metadata into the database
     *
     * @param options - Insert options
     * @returns The inserted entry with ID
     * @throws {ValidationError} If vector dimension is invalid
     * @throws {DatabaseError} If insertion fails
     */
    insert(options: MemoryInsertOptions): Promise<{
        id: string;
        timestamp: number;
    }>;
    /**
     * Search for similar vectors using HNSW indexing
     *
     * @param query - Query vector
     * @param options - Search options
     * @returns Array of search results sorted by similarity (descending)
     * @throws {ValidationError} If query dimension is invalid
     */
    vectorSearch(query: Float32Array, options?: VectorSearchOptions): Promise<VectorSearchResult[]>;
    /**
     * Update a vector and/or its metadata
     *
     * @param options - Update options
     * @returns True if update succeeded
     * @throws {ValidationError} If vector dimension is invalid
     * @throws {DatabaseError} If vector not found
     */
    update(options: MemoryUpdateOptions): Promise<boolean>;
    /**
     * Delete a vector by ID
     *
     * @param options - Delete options
     * @returns True if deletion succeeded, false if not found
     */
    delete(options: MemoryDeleteOptions): Promise<boolean>;
    /**
     * Get a vector by ID
     *
     * @param options - Get options
     * @returns Vector entry or null if not found
     */
    get(options: MemoryGetOptions): Promise<VectorEntry | null>;
    /**
     * Insert multiple vectors in a batch
     *
     * @param entries - Array of entries to insert
     * @returns Batch insert result with success/failure counts
     */
    batchInsert(entries: MemoryInsertOptions[]): Promise<BatchInsertResult>;
    /**
     * Get database statistics
     *
     * @returns Database statistics including vector count and index info
     */
    getStats(): Promise<AgentDBStats>;
    /**
     * Close the database connection
     */
    close(): Promise<void>;
    /**
     * Get the underlying AgentDB instance (for advanced usage)
     *
     * @returns The raw AgentDB instance
     */
    getRawInstance(): AgentDB;
}
//# sourceMappingURL=agentdb-wrapper.d.ts.map