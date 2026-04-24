// Agent spawn tool implementation using FastMCP
import { z } from 'zod';
import { execSync } from 'child_process';
export const agentSpawnTool = {
    name: 'agent_spawn',
    description: 'Spawn a new agent in the swarm',
    parameters: z.object({
        type: z.enum(['researcher', 'coder', 'analyst', 'optimizer', 'coordinator'])
            .describe('Agent type'),
        capabilities: z.array(z.string())
            .optional()
            .describe('Agent capabilities'),
        name: z.string()
            .optional()
            .describe('Custom agent name')
    }),
    execute: async ({ type, capabilities, name }, { onProgress, auth }) => {
        try {
            const capStr = capabilities ? ` --capabilities "${capabilities.join(',')}"` : '';
            const nameStr = name ? ` --name "${name}"` : '';
            const cmd = `npx claude-flow@alpha agent spawn --type ${type}${capStr}${nameStr}`;
            const result = execSync(cmd, {
                encoding: 'utf-8',
                maxBuffer: 10 * 1024 * 1024
            });
            return {
                success: true,
                type,
                capabilities,
                name,
                result: result.trim(),
                userId: auth?.userId,
                timestamp: new Date().toISOString()
            };
        }
        catch (error) {
            throw new Error(`Failed to spawn agent: ${error.message}`);
        }
    }
};
//# sourceMappingURL=spawn.js.map