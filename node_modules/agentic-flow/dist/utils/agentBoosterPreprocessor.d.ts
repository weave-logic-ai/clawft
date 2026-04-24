/**
 * Agent Booster Pre-Processor
 *
 * Detects code editing intents in agent tasks and attempts Agent Booster
 * pattern matching before falling back to LLM.
 */
export interface EditIntent {
    type: string;
    task: string;
    filePath?: string;
    originalCode?: string;
    targetCode?: string;
    confidence: number;
}
export interface PreprocessorResult {
    success: boolean;
    method: 'agent_booster' | 'llm_required';
    output?: string;
    latency?: number;
    confidence?: number;
    strategy?: string;
    reason?: string;
}
export declare class AgentBoosterPreprocessor {
    private confidenceThreshold;
    private enabledIntents;
    constructor(options?: {
        confidenceThreshold?: number;
        enabledIntents?: string[];
    });
    /**
     * Detect if a task is a code editing intent that Agent Booster can handle
     */
    detectIntent(task: string): EditIntent | null;
    /**
     * Try to apply edit using Agent Booster
     */
    tryApply(intent: EditIntent): Promise<PreprocessorResult>;
    /**
     * Extract file path from task description
     */
    private extractFilePath;
    /**
     * Detect programming language from file extension
     */
    private detectLanguage;
    /**
     * Extract var to const transformation
     */
    private extractVarToConst;
    /**
     * Extract add types transformation (TypeScript)
     */
    private extractAddTypes;
    /**
     * Extract add error handling transformation
     */
    private extractAddErrorHandling;
    /**
     * Extract async/await transformation
     */
    private extractAsyncAwait;
    /**
     * Extract add logging transformation
     */
    private extractAddLogging;
    /**
     * Extract remove console transformation
     */
    private extractRemoveConsole;
}
//# sourceMappingURL=agentBoosterPreprocessor.d.ts.map