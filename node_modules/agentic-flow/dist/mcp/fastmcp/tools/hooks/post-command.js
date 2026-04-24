/**
 * Post-Command Hook - Learn from command outcomes
 * Records command results and learns from errors
 */
import { z } from 'zod';
import { loadIntelligence, saveIntelligence, simpleEmbed } from './shared.js';
const LEARNING_RATE = 0.1;
export const hookPostCommandTool = {
    name: 'hook_post_command',
    description: 'Post-command learning: record outcome and learn from errors',
    parameters: z.object({
        command: z.string().describe('Command that was executed'),
        exitCode: z.number().describe('Command exit code'),
        stderr: z.string().optional().describe('Standard error output'),
        stdout: z.string().optional().describe('Standard output (truncated)')
    }),
    execute: async ({ command, exitCode, stderr, stdout }, { onProgress }) => {
        const startTime = Date.now();
        const intel = loadIntelligence();
        const success = exitCode === 0;
        const cmdBase = command.split(' ')[0];
        const state = `command:${cmdBase}`;
        // 1. Update command patterns
        if (!intel.patterns[state]) {
            intel.patterns[state] = {};
        }
        const action = success ? 'success' : 'failure';
        const currentValue = intel.patterns[state][action] || 0;
        const reward = success ? 1.0 : -0.5;
        intel.patterns[state][action] = currentValue + LEARNING_RATE * (reward - currentValue);
        // 2. Learn from failures
        let errorPattern = null;
        if (!success && stderr) {
            // Extract error type from stderr
            const errorMatch = stderr.match(/^(\w+Error|\w+Exception|error\[\w+\]|ENOENT|EACCES|EPERM)/im);
            const errorType = errorMatch ? errorMatch[1] : 'CommandError';
            // Check for existing pattern
            const existing = intel.errorPatterns.find(p => p.errorType === errorType && p.context.includes(cmdBase));
            if (existing) {
                // Update existing pattern
                existing.agentSuccess['command'] = (existing.agentSuccess['command'] || 0) + 1;
            }
            else {
                // Create new error pattern
                errorPattern = {
                    errorType,
                    context: `${cmdBase} command: ${stderr.slice(0, 100)}`,
                    resolution: '', // Will be filled when resolved
                    agentSuccess: { 'command': 1 }
                };
                intel.errorPatterns.push(errorPattern);
            }
            // Keep last 50 error patterns
            if (intel.errorPatterns.length > 50) {
                intel.errorPatterns = intel.errorPatterns.slice(-50);
            }
        }
        // 3. Store successful command as memory if useful output
        if (success && stdout && stdout.length > 10) {
            const memoryContent = `Command "${cmdBase}" succeeded: ${stdout.slice(0, 100)}`;
            intel.memories.push({
                content: memoryContent,
                type: 'command',
                created: new Date().toISOString(),
                embedding: simpleEmbed(memoryContent)
            });
            // Keep last 200 memories
            if (intel.memories.length > 200) {
                intel.memories = intel.memories.slice(-200);
            }
        }
        // 4. Update metrics
        intel.metrics.totalRoutes++;
        if (success) {
            intel.metrics.successfulRoutes++;
        }
        // 5. Save intelligence
        saveIntelligence(intel);
        const latency = Date.now() - startTime;
        return {
            success: true,
            learned: true,
            commandSuccess: success,
            errorType: errorPattern?.errorType || null,
            patternValue: intel.patterns[state][action],
            latencyMs: latency,
            timestamp: new Date().toISOString()
        };
    }
};
//# sourceMappingURL=post-command.js.map