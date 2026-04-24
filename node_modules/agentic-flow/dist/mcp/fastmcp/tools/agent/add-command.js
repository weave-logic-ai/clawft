import { z } from 'zod';
import { writeFileSync, existsSync, mkdirSync } from 'fs';
import { join } from 'path';
const addCommandSchema = z.object({
    name: z.string().describe('Command name (kebab-case, e.g., custom-deploy)'),
    description: z.string().describe('Command description'),
    usage: z.string().describe('Command usage example'),
    parameters: z.array(z.object({
        name: z.string(),
        type: z.string(),
        required: z.boolean(),
        description: z.string()
    })).optional().describe('Command parameters'),
    examples: z.array(z.string()).optional().describe('Usage examples'),
    notes: z.string().optional().describe('Additional notes or warnings')
});
export const addCommandTool = {
    name: 'command_add',
    description: 'Add a new custom command defined in markdown format',
    parameters: addCommandSchema,
    async execute({ name, description, usage, parameters, examples, notes }, { onProgress }) {
        try {
            onProgress?.({ progress: 0.2, message: `Creating custom command: ${name}` });
            // Validate name format
            if (!/^[a-z0-9-]+$/.test(name)) {
                throw new Error('Command name must be kebab-case (lowercase, numbers, hyphens only)');
            }
            // Create commands directory if it doesn't exist
            const commandsDir = join(process.cwd(), '.claude', 'commands');
            if (!existsSync(commandsDir)) {
                mkdirSync(commandsDir, { recursive: true });
            }
            onProgress?.({ progress: 0.4, message: 'Generating command markdown...' });
            // Generate markdown content
            const markdown = `# ${name.split('-').map(w => w.charAt(0).toUpperCase() + w.slice(1)).join(' ')} Command

## Description
${description}

## Usage
\`\`\`bash
${usage}
\`\`\`

${parameters && parameters.length > 0 ? `## Parameters
| Name | Type | Required | Description |
|------|------|----------|-------------|
${parameters.map(p => `| \`${p.name}\` | ${p.type} | ${p.required ? 'Yes' : 'No'} | ${p.description} |`).join('\n')}
` : ''}

${examples && examples.length > 0 ? `## Examples

${examples.map((ex, i) => `### Example ${i + 1}
\`\`\`bash
${ex}
\`\`\`
`).join('\n')}` : ''}

${notes ? `## Notes
${notes}
` : ''}

## MCP Tool Usage
\`\`\`json
{
  "name": "command_execute",
  "arguments": {
    "command": "${name}",
    "args": []
  }
}
\`\`\`

---
*Generated: ${new Date().toISOString()}*
`;
            const filePath = join(commandsDir, `${name}.md`);
            // Check if command already exists
            if (existsSync(filePath)) {
                throw new Error(`Command '${name}' already exists at ${filePath}`);
            }
            onProgress?.({ progress: 0.7, message: 'Writing command file...' });
            // Write the markdown file
            writeFileSync(filePath, markdown, 'utf8');
            onProgress?.({ progress: 1.0, message: 'Command created successfully' });
            return {
                content: [{
                        type: 'text',
                        text: JSON.stringify({
                            success: true,
                            command: name,
                            filePath,
                            description,
                            parameters: parameters || [],
                            examples: examples || [],
                            message: `Command '${name}' created successfully at ${filePath}`,
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
                            command: name,
                            error: error.message,
                            timestamp: new Date().toISOString()
                        }, null, 2)
                    }],
                isError: true
            };
        }
    }
};
//# sourceMappingURL=add-command.js.map