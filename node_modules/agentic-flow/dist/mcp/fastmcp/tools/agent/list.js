import { z } from 'zod';
import { execSync } from 'child_process';
const listAgentsSchema = z.object({
    format: z.enum(['summary', 'detailed', 'json']).optional().default('summary')
        .describe('Output format for agent list')
});
export const listAgentsTool = {
    name: 'agent_list',
    description: 'List all available agents (equivalent to --list CLI command)',
    parameters: listAgentsSchema,
    async execute({ format }, { onProgress }) {
        try {
            onProgress?.({ progress: 0.3, message: 'Loading available agents...' });
            // Execute list command
            const result = execSync('npx agentic-flow --list', {
                encoding: 'utf8',
                maxBuffer: 5 * 1024 * 1024,
                timeout: 30000
            });
            onProgress?.({ progress: 0.8, message: 'Processing agent list...' });
            // Parse the output to extract agent info
            const agents = [];
            const lines = result.split('\n');
            let currentCategory = '';
            for (const line of lines) {
                if (line.includes(':') && line.trim().endsWith(':')) {
                    currentCategory = line.replace(':', '').trim();
                }
                else if (line.trim().startsWith('•') || /^\s{2,}\w/.test(line)) {
                    const match = line.match(/^\s*[•\s]*(\S+)\s+(.+)$/);
                    if (match) {
                        agents.push({
                            name: match[1],
                            description: match[2].trim(),
                            category: currentCategory
                        });
                    }
                }
            }
            onProgress?.({ progress: 1.0, message: `Found ${agents.length} agents` });
            let output;
            if (format === 'json') {
                output = JSON.stringify({ agents, count: agents.length }, null, 2);
            }
            else if (format === 'detailed') {
                output = result;
            }
            else {
                output = JSON.stringify({
                    success: true,
                    count: agents.length,
                    agents: agents.map(a => ({
                        name: a.name,
                        category: a.category,
                        description: a.description.substring(0, 100) + (a.description.length > 100 ? '...' : '')
                    })),
                    timestamp: new Date().toISOString()
                }, null, 2);
            }
            return {
                content: [{
                        type: 'text',
                        text: output
                    }]
            };
        }
        catch (error) {
            return {
                content: [{
                        type: 'text',
                        text: JSON.stringify({
                            success: false,
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
//# sourceMappingURL=list.js.map