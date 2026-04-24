export interface ToolInstructions {
    format: string;
    commands: {
        write: string;
        read: string;
        bash: string;
    };
    examples?: string;
    emphasis?: string;
}
export declare const BASE_INSTRUCTIONS: ToolInstructions;
export declare const ANTHROPIC_INSTRUCTIONS: ToolInstructions;
export declare const OPENAI_INSTRUCTIONS: ToolInstructions;
export declare const GOOGLE_INSTRUCTIONS: ToolInstructions;
export declare const META_INSTRUCTIONS: ToolInstructions;
export declare const DEEPSEEK_INSTRUCTIONS: ToolInstructions;
export declare const MISTRAL_INSTRUCTIONS: ToolInstructions;
export declare const XAI_INSTRUCTIONS: ToolInstructions;
export declare function getInstructionsForModel(modelId: string, provider?: string): ToolInstructions;
export declare function taskRequiresFileOps(systemPrompt: string, userMessages: any[]): boolean;
export declare function formatInstructions(instructions: ToolInstructions, includeXmlInstructions?: boolean): string;
export declare const PARALLEL_EXECUTION_INSTRUCTIONS = "\n\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\nCRITICAL: Concurrent Execution System\n\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\n\nThis system supports parallel agent execution via CLI subprocesses and\nReasoningBank coordination. Follow these patterns for optimal performance:\n\n1. SUBPROCESS SPAWNING\n   When decomposing tasks, spawn subagents via CLI:\n\n   ```typescript\n   const results = await Promise.all([\n     exec('npx agentic-flow --agent researcher --task \"analyze security\"'),\n     exec('npx agentic-flow --agent coder --task \"implement features\"'),\n     exec('npx agentic-flow --agent tester --task \"create tests\"')\n   ]);\n   ```\n\n2. REASONINGBANK COORDINATION\n   Each subagent stores results in ReasoningBank:\n\n   ```typescript\n   // Subagent stores results\n   await reasoningBank.storePattern({\n     sessionId: 'swarm/task-123/agent-1',\n     task: 'Research security patterns',\n     output: findings,\n     reward: 0.95,\n     success: true\n   });\n\n   // Parent retrieves all results\n   const allResults = await reasoningBank.searchPatterns('swarm/task-123');\n   ```\n\n3. PERFORMANCE BENEFITS\n   - 2.8-4.4x speedup with parallel execution\n   - 50-70% time reduction with QUIC transport\n   - 32.3% token reduction via batching\n\nSee /agentic-flow/src/prompts/parallel-execution-guide.md for detailed examples.\n\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\n";
export interface InstructionOptions {
    enableParallel?: boolean;
    batchSize?: number;
    enableReasoningBank?: boolean;
    includeXmlInstructions?: boolean;
}
export declare function getMaxTokensForModel(modelId: string, requestedMaxTokens?: number): number;
export declare function getParallelCapabilities(modelId: string): {
    maxConcurrency: number;
    recommendedBatchSize: number;
    supportsSubprocesses: boolean;
    supportsReasoningBank: boolean;
};
export declare function buildInstructions(modelId: string, provider: string | undefined, options?: InstructionOptions): string;
//# sourceMappingURL=provider-instructions.d.ts.map