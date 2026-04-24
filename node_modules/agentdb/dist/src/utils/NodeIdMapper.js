/**
 * NodeIdMapper - Maps numeric episode IDs to full graph node IDs
 *
 * Solves the issue where ReflexionMemory.storeEpisode() returns numeric IDs
 * but GraphDatabaseAdapter edges need full string node IDs (e.g., "episode-123456")
 */
export class NodeIdMapper {
    static instance = null;
    numericToNode = new Map();
    nodeToNumeric = new Map();
    constructor() {
        // Private constructor for singleton pattern
    }
    static getInstance() {
        if (!NodeIdMapper.instance) {
            NodeIdMapper.instance = new NodeIdMapper();
        }
        return NodeIdMapper.instance;
    }
    /**
     * Register a mapping between numeric ID and full node ID
     */
    register(numericId, nodeId) {
        this.numericToNode.set(numericId, nodeId);
        this.nodeToNumeric.set(nodeId, numericId);
    }
    /**
     * Get full node ID from numeric ID
     */
    getNodeId(numericId) {
        return this.numericToNode.get(numericId);
    }
    /**
     * Get numeric ID from full node ID
     */
    getNumericId(nodeId) {
        return this.nodeToNumeric.get(nodeId);
    }
    /**
     * Clear all mappings (useful for testing)
     */
    clear() {
        this.numericToNode.clear();
        this.nodeToNumeric.clear();
    }
    /**
     * Get statistics about mappings
     */
    getStats() {
        return {
            totalMappings: this.numericToNode.size,
            numericIds: this.numericToNode.size,
            nodeIds: this.nodeToNumeric.size
        };
    }
}
//# sourceMappingURL=NodeIdMapper.js.map