/**
 * Tool Emulation Layer for Models Without Native Function Calling
 *
 * Implements two strategies:
 * 1. ReAct Pattern - Structured reasoning with tool use
 * 2. Prompt-Based - Direct JSON tool invocation
 *
 * Automatically selected based on model capabilities.
 */
/**
 * ReAct Pattern Implementation
 * Best for: Models with 32k+ context, complex multi-step tasks
 */
export class ReActEmulator {
    tools;
    constructor(tools) {
        this.tools = tools;
    }
    /**
     * Build ReAct prompt with tool catalog
     */
    buildPrompt(userMessage, previousSteps = '') {
        const toolCatalog = this.tools.map(tool => {
            const params = tool.input_schema?.properties || {};
            const required = tool.input_schema?.required || [];
            const paramDocs = Object.entries(params).map(([name, schema]) => {
                const req = required.includes(name) ? '(required)' : '(optional)';
                const type = schema.type || 'any';
                return `    - ${name} ${req}: ${type} - ${schema.description || ''}`;
            }).join('\n');
            return `• ${tool.name}: ${tool.description || 'No description'}
${paramDocs}`;
        }).join('\n\n');
        return `You are solving a task using available tools. Think step-by-step using this format:

Thought: [Your reasoning about what to do next]
Action: [tool_name]
Action Input: [JSON object with tool parameters]
Observation: [Tool result will be inserted here by the system]
... (repeat Thought/Action/Observation as needed)
Final Answer: [Your complete answer to the user's question]

Available Tools:
${toolCatalog}

IMPORTANT:
- Action Input must be valid JSON matching the tool's schema
- Only use tools from the list above
- When you have enough information, provide a Final Answer
- If a tool fails, think about alternative approaches

${previousSteps}

User Question: ${userMessage}

Begin!`;
    }
    /**
     * Parse ReAct response and extract tool calls
     */
    parseResponse(response) {
        // Extract components ([\s\S] instead of /s flag for ES5 compatibility)
        const thoughtMatch = response.match(/Thought:\s*([\s\S]+?)(?=\n(?:Action:|Final Answer:|$))/);
        const actionMatch = response.match(/Action:\s*(\w+)/);
        const inputMatch = response.match(/Action Input:\s*(\{[\s\S]*?\})/);
        const finalMatch = response.match(/Final Answer:\s*([\s\S]+?)$/);
        if (finalMatch) {
            return {
                finalAnswer: finalMatch[1].trim(),
                thought: thoughtMatch?.[1].trim()
            };
        }
        if (actionMatch && inputMatch) {
            try {
                const args = JSON.parse(inputMatch[1].trim());
                return {
                    toolCall: {
                        name: actionMatch[1],
                        arguments: args,
                        id: `react_${Date.now()}`
                    },
                    thought: thoughtMatch?.[1].trim()
                };
            }
            catch (e) {
                console.error('Failed to parse Action Input JSON:', e);
                return { thought: thoughtMatch?.[1].trim() };
            }
        }
        return { thought: thoughtMatch?.[1].trim() };
    }
    /**
     * Build prompt with observation after tool execution
     */
    appendObservation(previousPrompt, observation) {
        return `${previousPrompt}\nObservation: ${observation}\n`;
    }
}
/**
 * Prompt-Based Tool Emulation
 * Best for: Simple tasks, models with limited context
 */
export class PromptEmulator {
    tools;
    constructor(tools) {
        this.tools = tools;
    }
    /**
     * Build simple prompt for tool invocation
     */
    buildPrompt(userMessage) {
        const toolCatalog = this.tools.map(tool => {
            const params = tool.input_schema?.properties || {};
            const paramList = Object.keys(params).join(', ');
            return `${tool.name}(${paramList}): ${tool.description || 'No description'}`;
        }).join('\n');
        return `You have access to these tools:

${toolCatalog}

To use a tool, respond with ONLY this JSON format (no other text):
{
  "tool": "tool_name",
  "arguments": {
    "param1": "value1",
    "param2": "value2"
  }
}

If you don't need a tool, respond with your answer normally.

User: ${userMessage}

Response:`;
    }
    /**
     * Parse response - either tool call JSON or regular text
     */
    parseResponse(response) {
        // Try to extract JSON from response
        const jsonMatch = response.match(/\{[\s\S]*\}/);
        if (!jsonMatch) {
            return { textResponse: response.trim() };
        }
        try {
            const parsed = JSON.parse(jsonMatch[0]);
            if (parsed.tool && parsed.arguments) {
                return {
                    toolCall: {
                        name: parsed.tool,
                        arguments: parsed.arguments,
                        id: `prompt_${Date.now()}`
                    }
                };
            }
        }
        catch (e) {
            // Not valid JSON, treat as text response
        }
        return { textResponse: response.trim() };
    }
}
/**
 * Unified Tool Emulation Interface
 */
export class ToolEmulator {
    tools;
    strategy;
    reactEmulator;
    promptEmulator;
    constructor(tools, strategy) {
        this.tools = tools;
        this.strategy = strategy;
        this.reactEmulator = new ReActEmulator(tools);
        this.promptEmulator = new PromptEmulator(tools);
    }
    /**
     * Build prompt based on selected strategy
     */
    buildPrompt(userMessage, context) {
        if (this.strategy === 'react') {
            return this.reactEmulator.buildPrompt(userMessage, context?.previousSteps);
        }
        else {
            return this.promptEmulator.buildPrompt(userMessage);
        }
    }
    /**
     * Parse model response and extract tool calls
     */
    parseResponse(response) {
        if (this.strategy === 'react') {
            return this.reactEmulator.parseResponse(response);
        }
        else {
            return this.promptEmulator.parseResponse(response);
        }
    }
    /**
     * Append observation (ReAct only)
     */
    appendObservation(prompt, observation) {
        if (this.strategy === 'react') {
            return this.reactEmulator.appendObservation(prompt, observation);
        }
        return prompt;
    }
    /**
     * Validate tool call against schema
     */
    validateToolCall(toolCall) {
        const tool = this.tools.find(t => t.name === toolCall.name);
        if (!tool) {
            return {
                valid: false,
                errors: [`Tool '${toolCall.name}' not found. Available: ${this.tools.map(t => t.name).join(', ')}`]
            };
        }
        const errors = [];
        const schema = tool.input_schema;
        if (!schema) {
            return { valid: true }; // No schema to validate against
        }
        // Check required parameters
        const required = schema.required || [];
        for (const param of required) {
            if (!(param in toolCall.arguments)) {
                errors.push(`Missing required parameter: ${param}`);
            }
        }
        // Type checking (basic)
        const properties = schema.properties || {};
        for (const [key, value] of Object.entries(toolCall.arguments)) {
            if (properties[key]) {
                const expectedType = properties[key].type;
                const actualType = typeof value;
                if (expectedType === 'string' && actualType !== 'string') {
                    errors.push(`Parameter '${key}' must be string, got ${actualType}`);
                }
                else if (expectedType === 'number' && actualType !== 'number') {
                    errors.push(`Parameter '${key}' must be number, got ${actualType}`);
                }
                else if (expectedType === 'boolean' && actualType !== 'boolean') {
                    errors.push(`Parameter '${key}' must be boolean, got ${actualType}`);
                }
            }
        }
        return {
            valid: errors.length === 0,
            errors: errors.length > 0 ? errors : undefined
        };
    }
    /**
     * Get confidence score for emulation result
     * Based on: JSON validity, schema compliance, reasoning quality
     */
    getConfidence(parsed) {
        let confidence = 0.5; // Base confidence
        if (parsed.toolCall) {
            const validation = this.validateToolCall(parsed.toolCall);
            if (validation.valid) {
                confidence += 0.3; // Valid tool call
            }
            else {
                confidence -= 0.2; // Invalid tool call
            }
        }
        if (parsed.thought && parsed.thought.length > 20) {
            confidence += 0.1; // Good reasoning
        }
        if (parsed.finalAnswer && parsed.finalAnswer.length > 10) {
            confidence += 0.1; // Complete answer
        }
        return Math.max(0, Math.min(1, confidence));
    }
}
/**
 * Execute tool emulation loop
 */
export async function executeEmulation(emulator, userMessage, modelCall, toolExecutor, options = {}) {
    const maxIterations = options.maxIterations || 5;
    const verbose = options.verbose || false;
    let prompt = emulator.buildPrompt(userMessage);
    const toolCalls = [];
    let finalAnswer;
    let lastReasoning;
    if (verbose) {
        console.log('\n🔧 Starting tool emulation...\n');
    }
    for (let i = 0; i < maxIterations; i++) {
        if (verbose) {
            console.log(`\n━━━ Iteration ${i + 1}/${maxIterations} ━━━`);
        }
        // Call model
        const response = await modelCall(prompt);
        if (verbose) {
            console.log(`Model response:\n${response.substring(0, 300)}...\n`);
        }
        // Parse response
        const parsed = emulator.parseResponse(response);
        if (parsed.finalAnswer) {
            finalAnswer = parsed.finalAnswer;
            lastReasoning = parsed.thought;
            if (verbose) {
                console.log('✅ Received final answer, stopping loop.\n');
            }
            break;
        }
        if (parsed.toolCall) {
            // Validate tool call
            const validation = emulator.validateToolCall(parsed.toolCall);
            if (!validation.valid) {
                if (verbose) {
                    console.log(`❌ Invalid tool call: ${validation.errors?.join(', ')}\n`);
                }
                // Append error as observation and retry
                prompt = emulator.appendObservation(prompt, `ERROR: ${validation.errors?.join('. ')}`);
                continue;
            }
            if (verbose) {
                console.log(`🔨 Executing tool: ${parsed.toolCall.name}`);
                console.log(`   Arguments: ${JSON.stringify(parsed.toolCall.arguments)}\n`);
            }
            // Execute tool
            try {
                const result = await toolExecutor(parsed.toolCall);
                toolCalls.push(parsed.toolCall);
                if (verbose) {
                    console.log(`✅ Tool result: ${JSON.stringify(result).substring(0, 200)}\n`);
                }
                // Append observation
                prompt = emulator.appendObservation(prompt, JSON.stringify(result));
            }
            catch (error) {
                if (verbose) {
                    console.log(`❌ Tool execution failed: ${error.message}\n`);
                }
                prompt = emulator.appendObservation(prompt, `ERROR: ${error.message}`);
            }
            lastReasoning = parsed.thought;
        }
        else if (parsed.textResponse) {
            // Model didn't use a tool and gave a direct response
            finalAnswer = parsed.textResponse;
            if (verbose) {
                console.log('📝 Model provided direct text response (no tool use).\n');
            }
            break;
        }
    }
    if (!finalAnswer && verbose) {
        console.log('⚠️  Reached max iterations without final answer.\n');
    }
    const confidence = emulator.getConfidence({
        toolCall: toolCalls[toolCalls.length - 1],
        finalAnswer,
        thought: lastReasoning
    });
    return {
        toolCalls,
        reasoning: lastReasoning,
        finalAnswer,
        confidence
    };
}
//# sourceMappingURL=tool-emulation.js.map