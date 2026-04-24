import { AgentDefinition } from "../utils/agentLoader.js";
export declare function claudeAgent(agent: AgentDefinition, input: string, onStream?: (chunk: string) => void, modelOverride?: string): Promise<{
    output: string;
    agent: string;
}>;
//# sourceMappingURL=claudeAgent.d.ts.map