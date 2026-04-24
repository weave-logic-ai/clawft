/**
 * ReasoningBank WASM Adapter
 *
 * TypeScript wrapper for the Rust WASM ReasoningBank implementation.
 * Provides browser and Node.js compatibility with auto-detection of storage backends.
 *
 * Features:
 * - IndexedDB storage (browser preferred)
 * - sql.js fallback (browser backup)
 * - Native performance via WASM
 * - Async/await API
 * - Type-safe interfaces
 */
export interface PatternInput {
    task_description: string;
    task_category: string;
    strategy: string;
    success_score: number;
    duration_seconds?: number;
}
export interface Pattern {
    id: string;
    task_description: string;
    task_category: string;
    strategy: string;
    success_score?: number;
    duration_seconds?: number;
    created_at: string;
    updated_at?: string;
}
export interface SimilarPattern {
    pattern: Pattern;
    similarity_score: number;
}
export interface StorageStats {
    total_patterns: number;
    categories: number;
    avg_success_score: number;
    storage_backend: string;
}
/**
 * ReasoningBank WASM Adapter
 *
 * Provides a TypeScript-friendly interface to the Rust WASM implementation.
 * Automatically selects the best storage backend for the environment.
 */
export declare class ReasoningBankAdapter {
    private wasmInstance;
    private initPromise;
    /**
     * Create a new ReasoningBank instance
     *
     * @param dbName - Optional database name (default: "reasoningbank")
     */
    constructor(dbName?: string);
    private initialize;
    private ensureInitialized;
    /**
     * Store a reasoning pattern
     *
     * @param pattern - Pattern input data
     * @returns Pattern UUID
     */
    storePattern(pattern: PatternInput): Promise<string>;
    /**
     * Retrieve a pattern by ID
     *
     * @param id - Pattern UUID
     * @returns Pattern or null if not found
     */
    getPattern(id: string): Promise<Pattern | null>;
    /**
     * Search patterns by category
     *
     * @param category - Task category
     * @param limit - Maximum number of results (default: 10)
     * @returns Array of patterns
     */
    searchByCategory(category: string, limit?: number): Promise<Pattern[]>;
    /**
     * Find similar patterns
     *
     * @param taskDescription - Task description to match
     * @param taskCategory - Task category
     * @param topK - Number of similar patterns to return (default: 5)
     * @returns Array of similar patterns with similarity scores
     */
    findSimilar(taskDescription: string, taskCategory: string, topK?: number): Promise<SimilarPattern[]>;
    /**
     * Get storage statistics
     *
     * @returns Storage stats including total patterns, categories, and backend info
     */
    getStats(): Promise<StorageStats>;
}
/**
 * Create a new ReasoningBank instance
 *
 * @param dbName - Optional database name
 * @returns ReasoningBank adapter instance
 */
export declare function createReasoningBank(dbName?: string): Promise<ReasoningBankAdapter>;
export default ReasoningBankAdapter;
//# sourceMappingURL=wasm-adapter.d.ts.map