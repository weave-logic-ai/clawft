/**
 * NodeIdMapper - Maps numeric episode IDs to full graph node IDs
 *
 * Solves the issue where ReflexionMemory.storeEpisode() returns numeric IDs
 * but GraphDatabaseAdapter edges need full string node IDs (e.g., "episode-123456")
 */

export class NodeIdMapper {
  private static instance: NodeIdMapper | null = null;
  private numericToNode = new Map<number, string>();
  private nodeToNumeric = new Map<string, number>();

  private constructor() {
    // Private constructor for singleton pattern
  }

  static getInstance(): NodeIdMapper {
    if (!NodeIdMapper.instance) {
      NodeIdMapper.instance = new NodeIdMapper();
    }
    return NodeIdMapper.instance;
  }

  /**
   * Register a mapping between numeric ID and full node ID
   */
  register(numericId: number, nodeId: string): void {
    this.numericToNode.set(numericId, nodeId);
    this.nodeToNumeric.set(nodeId, numericId);
  }

  /**
   * Get full node ID from numeric ID
   */
  getNodeId(numericId: number): string | undefined {
    return this.numericToNode.get(numericId);
  }

  /**
   * Get numeric ID from full node ID
   */
  getNumericId(nodeId: string): number | undefined {
    return this.nodeToNumeric.get(nodeId);
  }

  /**
   * Clear all mappings (useful for testing)
   */
  clear(): void {
    this.numericToNode.clear();
    this.nodeToNumeric.clear();
  }

  /**
   * Get statistics about mappings
   */
  getStats(): { totalMappings: number; numericIds: number; nodeIds: number } {
    return {
      totalMappings: this.numericToNode.size,
      numericIds: this.numericToNode.size,
      nodeIds: this.nodeToNumeric.size
    };
  }
}
