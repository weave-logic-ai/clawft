import { z } from 'zod';
import { execSync } from 'child_process';
const parallelModeSchema = z.object({
    topic: z.string().optional().describe('Research topic for parallel mode'),
    diff: z.string().optional().describe('Code diff for review'),
    dataset: z.string().optional().describe('Dataset hint for analysis'),
    streaming: z.boolean().optional().default(false).describe('Enable streaming output')
});
export const parallelModeTool = {
    name: 'agent_parallel',
    description: 'Run parallel mode with 3 agents (research, code review, data analysis)',
    parameters: parallelModeSchema,
    async execute({ topic, diff, dataset, streaming }, { onProgress }) {
        try {
            onProgress?.({ progress: 0.1, message: 'Starting parallel agent execution' });
            // Set environment variables
            const env = {
                ...process.env,
                ...(topic && { TOPIC: topic }),
                ...(diff && { DIFF: diff }),
                ...(dataset && { DATASET: dataset }),
                ...(streaming && { ENABLE_STREAMING: 'true' })
            };
            onProgress?.({ progress: 0.3, message: 'Executing 3 agents in parallel...' });
            // Execute parallel mode
            const result = execSync('npx agentic-flow', {
                encoding: 'utf8',
                maxBuffer: 10 * 1024 * 1024,
                timeout: 300000,
                env
            });
            onProgress?.({ progress: 1.0, message: 'Parallel execution completed' });
            return {
                content: [{
                        type: 'text',
                        text: JSON.stringify({
                            success: true,
                            mode: 'parallel',
                            agents: ['research', 'code_review', 'data'],
                            config: { topic, diff, dataset, streaming },
                            output: result,
                            timestamp: new Date().toISOString()
                        }, null, 2)
                    }]
            };
        }
        catch (error) {
            return {
                content: [{
                        type: 'text',
                        text: JSON.stringify({
                            success: false,
                            mode: 'parallel',
                            error: error.message,
                            stderr: error.stderr?.toString(),
                            timestamp: new Date().toISOString()
                        }, null, 2)
                    }],
                isError: true
            };
        }
    }
};
//# sourceMappingURL=parallel.js.map