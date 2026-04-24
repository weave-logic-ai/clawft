// Task orchestration tool implementation using FastMCP
import { z } from 'zod';
import { execSync } from 'child_process';
export const taskOrchestrateTool = {
    name: 'task_orchestrate',
    description: 'Orchestrate a complex task across the swarm',
    parameters: z.object({
        task: z.string()
            .min(1)
            .describe('Task description or instructions'),
        strategy: z.enum(['parallel', 'sequential', 'adaptive'])
            .optional()
            .default('adaptive')
            .describe('Execution strategy'),
        priority: z.enum(['low', 'medium', 'high', 'critical'])
            .optional()
            .default('medium')
            .describe('Task priority'),
        maxAgents: z.number()
            .positive()
            .optional()
            .describe('Maximum agents to use for this task')
    }),
    execute: async ({ task, strategy, priority, maxAgents }, { onProgress, auth }) => {
        try {
            const maxStr = maxAgents ? ` --max-agents ${maxAgents}` : '';
            const cmd = `npx claude-flow@alpha task orchestrate "${task}" --strategy ${strategy} --priority ${priority}${maxStr}`;
            const result = execSync(cmd, {
                encoding: 'utf-8',
                maxBuffer: 10 * 1024 * 1024
            });
            return {
                success: true,
                task,
                strategy,
                priority,
                maxAgents,
                result: result.trim(),
                userId: auth?.userId,
                timestamp: new Date().toISOString()
            };
        }
        catch (error) {
            throw new Error(`Failed to orchestrate task: ${error.message}`);
        }
    }
};
//# sourceMappingURL=orchestrate.js.map