"use strict";
/**
 * ruvector - High-performance vector database for Node.js
 *
 * This package automatically detects and uses the best available implementation:
 * 1. Native (Rust-based, fastest) - if available for your platform
 * 2. RVF (persistent store) - if @ruvector/rvf is installed
 * 3. Stub (testing fallback) - limited functionality
 *
 * Also provides safe wrappers for GNN and Attention modules that handle
 * array type conversions automatically.
 */
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __exportStar = (this && this.__exportStar) || function(m, exports) {
    for (var p in m) if (p !== "default" && !Object.prototype.hasOwnProperty.call(exports, p)) __createBinding(exports, m, p);
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.NativeVectorDb = exports.VectorDB = exports.VectorDb = void 0;
exports.getImplementationType = getImplementationType;
exports.isNative = isNative;
exports.isRvf = isRvf;
exports.isWasm = isWasm;
exports.getVersion = getVersion;
__exportStar(require("./types"), exports);
// Export core wrappers (safe interfaces with automatic type conversion)
__exportStar(require("./core"), exports);
__exportStar(require("./services"), exports);
let implementation;
let implementationType = 'wasm';
// Check for explicit --backend rvf flag or RUVECTOR_BACKEND env var
const rvfRequested = process.env.RUVECTOR_BACKEND === 'rvf' ||
    process.argv.includes('--backend') && process.argv[process.argv.indexOf('--backend') + 1] === 'rvf';
if (rvfRequested) {
    // Explicit rvf backend requested - fail hard if not available
    try {
        implementation = require('@ruvector/rvf');
        implementationType = 'rvf';
    }
    catch (e) {
        throw new Error('@ruvector/rvf is not installed.\n' +
            '  Run: npm install @ruvector/rvf\n' +
            '  The --backend rvf flag requires this package.');
    }
}
else {
    try {
        // Try to load native module first
        implementation = require('@ruvector/core');
        implementationType = 'native';
        // Verify it's actually working (native module exports VectorDb, not VectorDB)
        if (typeof implementation.VectorDb !== 'function') {
            throw new Error('Native module loaded but VectorDb class not found');
        }
    }
    catch (e) {
        // Try rvf (persistent store) as second fallback
        try {
            implementation = require('@ruvector/rvf');
            implementationType = 'rvf';
        }
        catch (rvfErr) {
            // Graceful fallback - don't crash, just warn
            console.warn('[RuVector] Native module not available:', e.message);
            console.warn('[RuVector] RVF module not available:', rvfErr.message);
            console.warn('[RuVector] Vector operations will be limited. Install @ruvector/core or @ruvector/rvf for full functionality.');
            // Create a stub implementation that provides basic functionality
            implementation = {
                VectorDb: class StubVectorDb {
                    constructor() {
                        console.warn('[RuVector] Using stub VectorDb - install @ruvector/core for native performance');
                    }
                    async insert() { return 'stub-id-' + Date.now(); }
                    async insertBatch(entries) { return entries.map(() => 'stub-id-' + Date.now()); }
                    async search() { return []; }
                    async delete() { return true; }
                    async get() { return null; }
                    async len() { return 0; }
                    async isEmpty() { return true; }
                }
            };
            implementationType = 'wasm'; // Mark as fallback mode
        }
    }
}
/**
 * Get the current implementation type
 */
function getImplementationType() {
    return implementationType;
}
/**
 * Check if native implementation is being used
 */
function isNative() {
    return implementationType === 'native';
}
/**
 * Check if RVF implementation is being used
 */
function isRvf() {
    return implementationType === 'rvf';
}
/**
 * Check if stub/fallback implementation is being used
 */
function isWasm() {
    return implementationType === 'wasm';
}
/**
 * Get version information
 */
function getVersion() {
    const pkg = require('../package.json');
    return {
        version: pkg.version,
        implementation: implementationType
    };
}
/**
 * Wrapper class that automatically handles metadata JSON conversion
 */
class VectorDBWrapper {
    constructor(options) {
        this.db = new implementation.VectorDb(options);
    }
    /**
     * Insert a vector with optional metadata (objects are auto-converted to JSON)
     */
    async insert(entry) {
        const nativeEntry = {
            id: entry.id,
            vector: entry.vector instanceof Float32Array ? entry.vector : new Float32Array(entry.vector),
        };
        // Auto-convert metadata object to JSON string
        if (entry.metadata) {
            nativeEntry.metadata = JSON.stringify(entry.metadata);
        }
        return this.db.insert(nativeEntry);
    }
    /**
     * Insert multiple vectors in batch
     */
    async insertBatch(entries) {
        const nativeEntries = entries.map(entry => ({
            id: entry.id,
            vector: entry.vector instanceof Float32Array ? entry.vector : new Float32Array(entry.vector),
            metadata: entry.metadata ? JSON.stringify(entry.metadata) : undefined,
        }));
        return this.db.insertBatch(nativeEntries);
    }
    /**
     * Search for similar vectors (metadata is auto-parsed from JSON)
     */
    async search(query) {
        const nativeQuery = {
            vector: query.vector instanceof Float32Array ? query.vector : new Float32Array(query.vector),
            k: query.k,
            efSearch: query.efSearch,
        };
        // Auto-convert filter object to JSON string
        if (query.filter) {
            nativeQuery.filter = JSON.stringify(query.filter);
        }
        const results = await this.db.search(nativeQuery);
        // Auto-parse metadata JSON strings back to objects
        return results.map((r) => ({
            id: r.id,
            score: r.score,
            vector: r.vector,
            metadata: r.metadata ? JSON.parse(r.metadata) : undefined,
        }));
    }
    /**
     * Get a vector by ID (metadata is auto-parsed from JSON)
     */
    async get(id) {
        const entry = await this.db.get(id);
        if (!entry)
            return null;
        return {
            id: entry.id,
            vector: entry.vector,
            metadata: entry.metadata ? JSON.parse(entry.metadata) : undefined,
        };
    }
    /**
     * Delete a vector by ID
     */
    async delete(id) {
        return this.db.delete(id);
    }
    /**
     * Get the number of vectors in the database
     */
    async len() {
        return this.db.len();
    }
    /**
     * Check if the database is empty
     */
    async isEmpty() {
        return this.db.isEmpty();
    }
}
// Export the wrapper class (aliased as VectorDB for backwards compatibility)
exports.VectorDb = VectorDBWrapper;
exports.VectorDB = VectorDBWrapper;
// Also export the raw native implementation for advanced users
exports.NativeVectorDb = implementation.VectorDb;
// Export everything from the implementation
exports.default = implementation;
