export interface ClaudeFlowAgentOptions {
    enableMemory?: boolean;
    enableCoordination?: boolean;
    memoryNamespace?: string;
    swarmTopology?: 'hierarchical' | 'mesh' | 'ring' | 'star';
    onStream?: (chunk: string) => void;
}
/**
 * Execute agent with Claude Flow memory and coordination
 */
export declare function claudeFlowAgent(agentName: string, systemPrompt: string, input: string, options?: ClaudeFlowAgentOptions): Promise<{
    output: string;
}>;
/**
 * Example: Memory-enabled research agent
 */
export declare function memoryResearchAgent(topic: string, onStream?: (chunk: string) => void): Promise<{
    output: string;
}>;
/**
 * Example: Coordination-enabled orchestrator agent
 */
export declare function orchestratorAgent(task: string, onStream?: (chunk: string) => void): Promise<{
    output: string;
}>;
/**
 * Example: Full-featured agent with memory and coordination
 */
export declare function hybridAgent(task: string, agentName?: string, systemPrompt?: string, onStream?: (chunk: string) => void): Promise<{
    output: string;
}>;
//# sourceMappingURL=claudeFlowAgent.d.ts.map