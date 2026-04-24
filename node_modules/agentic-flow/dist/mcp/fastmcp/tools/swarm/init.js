// Swarm init tool implementation using FastMCP
import { z } from 'zod';
import { execSync } from 'child_process';
export const swarmInitTool = {
    name: 'swarm_init',
    description: 'Initialize a multi-agent swarm with specified topology',
    parameters: z.object({
        topology: z.enum(['mesh', 'hierarchical', 'ring', 'star'])
            .describe('Swarm topology'),
        maxAgents: z.number()
            .positive()
            .optional()
            .default(8)
            .describe('Maximum number of agents'),
        strategy: z.enum(['balanced', 'specialized', 'adaptive'])
            .optional()
            .default('balanced')
            .describe('Agent distribution strategy')
    }),
    execute: async ({ topology, maxAgents, strategy }, { onProgress, auth }) => {
        try {
            const cmd = `npx claude-flow@alpha swarm init --topology ${topology} --max-agents ${maxAgents} --strategy ${strategy}`;
            const result = execSync(cmd, {
                encoding: 'utf-8',
                maxBuffer: 10 * 1024 * 1024
            });
            return {
                success: true,
                topology,
                maxAgents,
                strategy,
                result: result.trim(),
                userId: auth?.userId,
                timestamp: new Date().toISOString()
            };
        }
        catch (error) {
            throw new Error(`Failed to initialize swarm: ${error.message}`);
        }
    }
};
//# sourceMappingURL=init.js.map