// Tool configuration for Claude Agent SDK
import { getClaudeFlowTools, defaultClaudeFlowConfig } from './claudeFlow.js';
export const enabledTools = [
    'Read',
    'Write',
    'Edit',
    'Bash',
    'Glob',
    'Grep',
    'WebFetch',
    'WebSearch'
];
// Get claude-flow tools dynamically
const claudeFlowTools = getClaudeFlowTools(defaultClaudeFlowConfig);
export const toolConfig = {
    // Enable all standard tools for full capability
    enableAllTools: true,
    // MCP servers configuration
    mcpServers: {
        'claude-flow': {
            command: 'npx',
            args: ['claude-flow@alpha', 'mcp', 'start'],
            env: {
                CLAUDE_FLOW_MEMORY_ENABLED: 'true',
                CLAUDE_FLOW_COORDINATION_ENABLED: 'true'
            }
        }
    },
    // Permission mode
    permissionMode: 'default',
    // Additional enabled tools from claude-flow
    additionalTools: claudeFlowTools
};
//# sourceMappingURL=tools.js.map