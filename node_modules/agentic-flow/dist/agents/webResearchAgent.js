// src/agents/webResearchAgent.ts
import { query } from "@anthropic-ai/claude-agent-sdk";
import { logger } from "../utils/logger.js";
import { withRetry } from "../utils/retry.js";
import { toolConfig } from "../config/tools.js";
export async function webResearchAgent(input, onStream) {
    const startTime = Date.now();
    logger.info('Starting web research agent', { input: input.substring(0, 100) });
    return withRetry(async () => {
        const result = query({
            prompt: input,
            options: {
                systemPrompt: `You perform fast web-style reconnaissance and return a concise bullet list of findings.`,
                ...toolConfig
            }
        });
        let output = '';
        for await (const msg of result) {
            if (msg.type === 'assistant') {
                const chunk = msg.message.content?.map((c) => c.type === 'text' ? c.text : '').join('') || '';
                output += chunk;
                // Stream chunks in real-time
                if (onStream && chunk) {
                    onStream(chunk);
                }
            }
        }
        const duration = Date.now() - startTime;
        logger.info('Web research agent completed', {
            duration,
            outputLength: output.length
        });
        return { output };
    });
}
//# sourceMappingURL=webResearchAgent.js.map