export interface AgentDefinition {
    name: string;
    description: string;
    systemPrompt: string;
    color?: string;
    tools?: string[];
    filePath: string;
}
/**
 * Load all agents from .claude/agents directory with deduplication
 * Local agents (.claude/agents in CWD) override package agents
 */
export declare function loadAgents(agentsDir?: string): Map<string, AgentDefinition>;
/**
 * Get a specific agent by name
 */
export declare function getAgent(name: string, agentsDir?: string): AgentDefinition | undefined;
/**
 * List all available agents
 */
export declare function listAgents(agentsDir?: string): AgentDefinition[];
//# sourceMappingURL=agentLoader.d.ts.map