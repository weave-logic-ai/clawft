/**
 * GraphBackend Interface - Graph database capabilities (Optional)
 *
 * Provides property graph storage and Cypher-like query capabilities.
 * Available when @ruvector/graph-node is installed.
 *
 * Features:
 * - Node and relationship management
 * - Cypher query execution
 * - Graph traversal and pattern matching
 * - Integration with vector search
 */
/**
 * Type guard to check if an object implements GraphBackend
 */
export function isGraphBackend(obj) {
    return (typeof obj === 'object' &&
        obj !== null &&
        typeof obj.execute === 'function' &&
        typeof obj.createNode === 'function' &&
        typeof obj.getNode === 'function' &&
        typeof obj.updateNode === 'function' &&
        typeof obj.deleteNode === 'function' &&
        typeof obj.createRelationship === 'function' &&
        typeof obj.getRelationship === 'function' &&
        typeof obj.deleteRelationship === 'function' &&
        typeof obj.traverse === 'function' &&
        typeof obj.shortestPath === 'function' &&
        typeof obj.vectorSearch === 'function' &&
        typeof obj.getStats === 'function' &&
        typeof obj.clear === 'function');
}
//# sourceMappingURL=GraphBackend.js.map