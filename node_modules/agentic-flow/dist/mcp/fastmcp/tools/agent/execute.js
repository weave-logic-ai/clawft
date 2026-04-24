import { z } from 'zod';
import { execSync } from 'child_process';
const executeAgentSchema = z.object({
    agent: z.string().describe('Agent name to execute (e.g., coder, researcher, reviewer)'),
    task: z.string().describe('Task description for the agent'),
    stream: z.boolean().optional().default(false).describe('Enable real-time streaming output')
});
export const executeAgentTool = {
    name: 'agent_execute',
    description: 'Execute a specific agent with a task (equivalent to --agent CLI command)',
    parameters: executeAgentSchema,
    async execute({ agent, task, stream }, { onProgress }) {
        try {
            onProgress?.({ progress: 0.1, message: `Starting agent: ${agent}` });
            // Build command
            const streamFlag = stream ? '--stream' : '';
            const cmd = `npx agentic-flow --agent "${agent}" --task "${task}" ${streamFlag}`.trim();
            onProgress?.({ progress: 0.3, message: 'Executing agent...' });
            // Execute with timeout and capture output
            const result = execSync(cmd, {
                encoding: 'utf8',
                maxBuffer: 10 * 1024 * 1024, // 10MB buffer
                timeout: 300000, // 5 minute timeout
                env: { ...process.env }
            });
            onProgress?.({ progress: 1.0, message: 'Agent execution completed' });
            return {
                content: [{
                        type: 'text',
                        text: JSON.stringify({
                            success: true,
                            agent,
                            task: task.substring(0, 100),
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
                            agent,
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
//# sourceMappingURL=execute.js.map