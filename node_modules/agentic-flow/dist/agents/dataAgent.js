// src/agents/dataAgent.ts
import { query } from "@anthropic-ai/claude-agent-sdk";
import { logger } from "../utils/logger.js";
import { withRetry } from "../utils/retry.js";
import { toolConfig } from "../config/tools.js";
export async function dataAgent(input, onStream) {
    const startTime = Date.now();
    logger.info('Starting data agent', { input: input.substring(0, 100) });
    return withRetry(async () => {
        const result = query({
            prompt: input,
            options: {
                systemPrompt: `You analyze tabular data and produce a short brief with 3 key stats and 1 risk.`,
                ...toolConfig
            }
        });
        let output = '';
        for await (const msg of result) {
            if (msg.type === 'assistant') {
                const chunk = msg.message.content?.map((c) => c.type === 'text' ? c.text : '').join('') || '';
                output += chunk;
                if (onStream && chunk) {
                    onStream(chunk);
                }
            }
        }
        const duration = Date.now() - startTime;
        logger.info('Data agent completed', {
            duration,
            outputLength: output.length
        });
        return { output };
    });
}
//# sourceMappingURL=dataAgent.js.map