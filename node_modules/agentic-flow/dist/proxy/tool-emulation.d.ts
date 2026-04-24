/**
 * Tool Emulation Layer for Models Without Native Function Calling
 *
 * Implements two strategies:
 * 1. ReAct Pattern - Structured reasoning with tool use
 * 2. Prompt-Based - Direct JSON tool invocation
 *
 * Automatically selected based on model capabilities.
 */
export interface Tool {
    name: string;
    description?: string;
    input_schema?: {
        type: string;
        properties?: Record<string, any>;
        required?: string[];
    };
}
export interface ToolCall {
    name: string;
    arguments: Record<string, any>;
    id?: string;
}
export interface EmulationResult {
    toolCalls: ToolCall[];
    reasoning?: string;
    finalAnswer?: string;
    confidence: number;
}
/**
 * ReAct Pattern Implementation
 * Best for: Models with 32k+ context, complex multi-step tasks
 */
export declare class ReActEmulator {
    private tools;
    constructor(tools: Tool[]);
    /**
     * Build ReAct prompt with tool catalog
     */
    buildPrompt(userMessage: string, previousSteps?: string): string;
    /**
     * Parse ReAct response and extract tool calls
     */
    parseResponse(response: string): {
        toolCall?: ToolCall;
        thought?: string;
        finalAnswer?: string;
    };
    /**
     * Build prompt with observation after tool execution
     */
    appendObservation(previousPrompt: string, observation: string): string;
}
/**
 * Prompt-Based Tool Emulation
 * Best for: Simple tasks, models with limited context
 */
export declare class PromptEmulator {
    private tools;
    constructor(tools: Tool[]);
    /**
     * Build simple prompt for tool invocation
     */
    buildPrompt(userMessage: string): string;
    /**
     * Parse response - either tool call JSON or regular text
     */
    parseResponse(response: string): {
        toolCall?: ToolCall;
        textResponse?: string;
    };
}
/**
 * Unified Tool Emulation Interface
 */
export declare class ToolEmulator {
    private tools;
    private strategy;
    private reactEmulator;
    private promptEmulator;
    constructor(tools: Tool[], strategy: 'react' | 'prompt');
    /**
     * Build prompt based on selected strategy
     */
    buildPrompt(userMessage: string, context?: {
        previousSteps?: string;
    }): string;
    /**
     * Parse model response and extract tool calls
     */
    parseResponse(response: string): {
        toolCall?: ToolCall;
        finalAnswer?: string;
        thought?: string;
        textResponse?: string;
    };
    /**
     * Append observation (ReAct only)
     */
    appendObservation(prompt: string, observation: string): string;
    /**
     * Validate tool call against schema
     */
    validateToolCall(toolCall: ToolCall): {
        valid: boolean;
        errors?: string[];
    };
    /**
     * Get confidence score for emulation result
     * Based on: JSON validity, schema compliance, reasoning quality
     */
    getConfidence(parsed: ReturnType<typeof this.parseResponse>): number;
}
/**
 * Execute tool emulation loop
 */
export declare function executeEmulation(emulator: ToolEmulator, userMessage: string, modelCall: (prompt: string) => Promise<string>, toolExecutor: (toolCall: ToolCall) => Promise<any>, options?: {
    maxIterations?: number;
    verbose?: boolean;
}): Promise<EmulationResult>;
//# sourceMappingURL=tool-emulation.d.ts.map