/**
 * Pre-Command Hook - Command risk assessment
 * Assesses command safety and suggests alternatives
 */
import { z } from 'zod';
import { loadIntelligence, assessCommandRisk } from './shared.js';
// Safer alternatives for risky commands
const saferAlternatives = {
    'rm -rf': 'rm -ri (interactive) or move to trash',
    'sudo rm': 'Consider using specific permissions instead',
    'chmod 777': 'chmod 755 or more restrictive permissions',
    'curl | sh': 'Download script first, review, then execute',
    'wget | sh': 'Download script first, review, then execute'
};
export const hookPreCommandTool = {
    name: 'hook_pre_command',
    description: 'Pre-command intelligence: assess risk level and suggest alternatives',
    parameters: z.object({
        command: z.string().describe('Command to assess')
    }),
    execute: async ({ command }, { onProgress }) => {
        const startTime = Date.now();
        const intel = loadIntelligence();
        // 1. Assess risk level
        const riskLevel = assessCommandRisk(command);
        // 2. Find matching safer alternatives
        const suggestions = [];
        for (const [risky, safe] of Object.entries(saferAlternatives)) {
            if (command.includes(risky)) {
                suggestions.push(safe);
            }
        }
        // 3. Check if command matches known error patterns
        const warnings = [];
        const cmdBase = command.split(' ')[0];
        for (const ep of intel.errorPatterns) {
            if (ep.context.includes(cmdBase) && ep.resolution) {
                warnings.push(`Previous issue: ${ep.resolution}`);
            }
        }
        // 4. Determine approval status
        const approved = riskLevel < 0.7;
        const requiresConfirmation = riskLevel >= 0.5 && riskLevel < 0.9;
        const blocked = riskLevel >= 0.9;
        // 5. Categorize risk
        let riskCategory;
        if (riskLevel < 0.3)
            riskCategory = 'safe';
        else if (riskLevel < 0.6)
            riskCategory = 'caution';
        else if (riskLevel < 0.9)
            riskCategory = 'dangerous';
        else
            riskCategory = 'blocked';
        const latency = Date.now() - startTime;
        return {
            success: true,
            riskLevel,
            riskCategory,
            approved,
            requiresConfirmation,
            blocked,
            suggestions,
            warnings: warnings.slice(0, 2),
            latencyMs: latency,
            timestamp: new Date().toISOString()
        };
    }
};
//# sourceMappingURL=pre-command.js.map