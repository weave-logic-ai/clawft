/**
 * VectorBackend - Unified interface for vector database backends
 *
 * Provides abstraction over different vector search implementations
 * (RuVector, hnswlib-node) for AgentDB v2.
 *
 * Design:
 * - String-based IDs for all operations (backends handle label mapping internally)
 * - Consistent SearchResult format across backends
 * - Save/load with metadata persistence
 * - Backend-specific optimizations hidden behind interface
 */
export {};
//# sourceMappingURL=VectorBackend.js.map