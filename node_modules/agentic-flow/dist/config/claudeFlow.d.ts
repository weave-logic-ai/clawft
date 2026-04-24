export interface ClaudeFlowConfig {
    enableMemory: boolean;
    enableCoordination: boolean;
    enableSwarm: boolean;
    memoryNamespace?: string;
    coordinationTopology?: 'hierarchical' | 'mesh' | 'ring' | 'star';
}
export declare const defaultClaudeFlowConfig: ClaudeFlowConfig;
/**
 * Initialize claude-flow MCP tools for agent use
 */
export declare function getClaudeFlowTools(config?: ClaudeFlowConfig): string[];
/**
 * Check if claude-flow MCP is available
 */
export declare function isClaudeFlowAvailable(): Promise<boolean>;
/**
 * Initialize memory namespace for agent session
 */
export declare function getMemoryConfig(agentName?: string): {
    namespace: string;
    ttl: number;
    action: "store";
};
/**
 * Initialize swarm coordination for multi-agent tasks
 */
export declare function getSwarmConfig(topology?: ClaudeFlowConfig['coordinationTopology']): {
    topology: "mesh" | "hierarchical" | "ring" | "star";
    maxAgents: number;
    strategy: "balanced";
};
//# sourceMappingURL=claudeFlow.d.ts.map