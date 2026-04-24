// src/agents/codeReviewAgent.ts
import { query } from "@anthropic-ai/claude-agent-sdk";
import { logger } from "../utils/logger.js";
import { withRetry } from "../utils/retry.js";
import { toolConfig } from "../config/tools.js";
export async function codeReviewAgent(input, onStream) {
    const startTime = Date.now();
    logger.info('Starting code review agent', { input: input.substring(0, 100) });
    return withRetry(async () => {
        const result = query({
            prompt: input,
            options: {
                systemPrompt: `You review diffs and point out risks, complexity, and tests to add.`,
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
        logger.info('Code review agent completed', {
            duration,
            outputLength: output.length
        });
        return { output };
    });
}
//# sourceMappingURL=codeReviewAgent.js.map