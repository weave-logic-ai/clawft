import { AgentDefinition } from "../utils/agentLoader.js";
export declare function claudeAgentDirect(agent: AgentDefinition, input: string, onStream?: (chunk: string) => void, modelOverride?: string): Promise<{
    output: string;
    agent: string;
}>;
//# sourceMappingURL=claudeAgentDirect.d.ts.map