/**
 * NodeIdMapper - Maps numeric episode IDs to full graph node IDs
 *
 * Solves the issue where ReflexionMemory.storeEpisode() returns numeric IDs
 * but GraphDatabaseAdapter edges need full string node IDs (e.g., "episode-123456")
 */
export declare class NodeIdMapper {
    private static instance;
    private numericToNode;
    private nodeToNumeric;
    private constructor();
    static getInstance(): NodeIdMapper;
    /**
     * Register a mapping between numeric ID and full node ID
     */
    register(numericId: number, nodeId: string): void;
    /**
     * Get full node ID from numeric ID
     */
    getNodeId(numericId: number): string | undefined;
    /**
     * Get numeric ID from full node ID
     */
    getNumericId(nodeId: string): number | undefined;
    /**
     * Clear all mappings (useful for testing)
     */
    clear(): void;
    /**
     * Get statistics about mappings
     */
    getStats(): {
        totalMappings: number;
        numericIds: number;
        nodeIds: number;
    };
}
//# sourceMappingURL=NodeIdMapper.d.ts.map