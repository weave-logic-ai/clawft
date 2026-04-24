import { AgentDefinition } from '../utils/agentLoader.js';
/**
 * Direct API agent using Anthropic SDK with native tool calling
 * Bypasses Claude Agent SDK subprocess issues entirely
 */
export declare function directApiAgent(agent: AgentDefinition, input: string, onStream?: (chunk: string) => void): Promise<{
    output: string;
    agent: string;
}>;
//# sourceMappingURL=directApiAgent.d.ts.map