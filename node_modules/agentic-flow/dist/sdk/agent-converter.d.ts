/**
 * Agent Converter - Converts agentic-flow agent definitions to Claude Agent SDK format
 *
 * Takes agents defined in .claude/agents/ and converts them to SDK's agents option format
 * for native subagent support via the Task tool.
 */
import { AgentDefinition } from "../utils/agentLoader.js";
/**
 * SDK Agent Definition format (from Claude Agent SDK)
 */
export interface SDKAgentDefinition {
    description: string;
    prompt: string;
    tools?: string[];
    model?: 'sonnet' | 'opus' | 'haiku' | 'inherit';
}
/**
 * Convert a single agentic-flow agent to SDK format
 */
export declare function convertAgentToSdkFormat(agent: AgentDefinition): SDKAgentDefinition;
/**
 * Convert all loaded agents to SDK format (with caching)
 */
export declare function convertAllAgentsToSdkFormat(): Record<string, SDKAgentDefinition>;
/**
 * Invalidate agent cache (call after agent definitions change)
 */
export declare function invalidateAgentCache(): void;
/**
 * Get essential agents for most tasks
 */
export declare function getEssentialAgents(): Record<string, SDKAgentDefinition>;
/**
 * Get agents for a specific use case
 */
export declare function getAgentsForUseCase(useCase: 'code' | 'research' | 'review' | 'full'): Record<string, SDKAgentDefinition>;
/**
 * Merge custom agents with essential agents
 */
export declare function getMergedAgents(includeCustom?: boolean): Record<string, SDKAgentDefinition>;
//# sourceMappingURL=agent-converter.d.ts.map