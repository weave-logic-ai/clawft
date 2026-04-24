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
import * as ReasoningBankWasm from '../../wasm/reasoningbank/reasoningbank_wasm.js';
/**
 * ReasoningBank WASM Adapter
 *
 * Provides a TypeScript-friendly interface to the Rust WASM implementation.
 * Automatically selects the best storage backend for the environment.
 */
export class ReasoningBankAdapter {
    wasmInstance = null;
    initPromise;
    /**
     * Create a new ReasoningBank instance
     *
     * @param dbName - Optional database name (default: "reasoningbank")
     */
    constructor(dbName) {
        this.initPromise = this.initialize(dbName);
    }
    async initialize(dbName) {
        try {
            this.wasmInstance = new ReasoningBankWasm.ReasoningBankWasm(dbName || 'reasoningbank');
        }
        catch (error) {
            throw new Error(`Failed to initialize ReasoningBank WASM: ${error}`);
        }
    }
    async ensureInitialized() {
        await this.initPromise;
        if (!this.wasmInstance) {
            throw new Error('ReasoningBank WASM not initialized');
        }
        return this.wasmInstance;
    }
    /**
     * Store a reasoning pattern
     *
     * @param pattern - Pattern input data
     * @returns Pattern UUID
     */
    async storePattern(pattern) {
        const wasm = await this.ensureInitialized();
        const json = JSON.stringify(pattern);
        return await wasm.storePattern(json);
    }
    /**
     * Retrieve a pattern by ID
     *
     * @param id - Pattern UUID
     * @returns Pattern or null if not found
     */
    async getPattern(id) {
        const wasm = await this.ensureInitialized();
        try {
            const json = await wasm.getPattern(id);
            return json ? JSON.parse(json) : null;
        }
        catch (error) {
            if (error instanceof Error && error.message.includes('not found')) {
                return null;
            }
            throw error;
        }
    }
    /**
     * Search patterns by category
     *
     * @param category - Task category
     * @param limit - Maximum number of results (default: 10)
     * @returns Array of patterns
     */
    async searchByCategory(category, limit = 10) {
        const wasm = await this.ensureInitialized();
        const json = await wasm.searchByCategory(category, limit);
        return JSON.parse(json);
    }
    /**
     * Find similar patterns
     *
     * @param taskDescription - Task description to match
     * @param taskCategory - Task category
     * @param topK - Number of similar patterns to return (default: 5)
     * @returns Array of similar patterns with similarity scores
     */
    async findSimilar(taskDescription, taskCategory, topK = 5) {
        const wasm = await this.ensureInitialized();
        const json = await wasm.findSimilar(taskDescription, taskCategory, topK);
        return JSON.parse(json);
    }
    /**
     * Get storage statistics
     *
     * @returns Storage stats including total patterns, categories, and backend info
     */
    async getStats() {
        const wasm = await this.ensureInitialized();
        const json = await wasm.getStats();
        return JSON.parse(json);
    }
}
/**
 * Create a new ReasoningBank instance
 *
 * @param dbName - Optional database name
 * @returns ReasoningBank adapter instance
 */
export async function createReasoningBank(dbName) {
    const adapter = new ReasoningBankAdapter(dbName);
    await adapter.getStats(); // Ensure initialization
    return adapter;
}
export default ReasoningBankAdapter;
//# sourceMappingURL=wasm-adapter.js.map