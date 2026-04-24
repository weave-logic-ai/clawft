/**
 * Post-Edit Hook - Learn from edit outcomes
 * Updates Q-table patterns and stores successful patterns
 *
 * NOW WITH RUVECTOR INTELLIGENCE:
 * - Completes trajectory tracking for reinforcement learning
 * - Stores patterns in ReasoningBank via SONA
 * - Uses EWC++ to prevent catastrophic forgetting
 */
import { z } from 'zod';
import * as path from 'path';
import { loadIntelligence, saveIntelligence, simpleEmbed } from './shared.js';
import { recordTrajectoryStep, endTaskTrajectory, storePattern, getIntelligenceStats } from './intelligence-bridge.js';
import { activeEditTrajectories } from './pre-edit.js';
const LEARNING_RATE = 0.1;
const SUCCESS_REWARD = 1.0;
const FAILURE_PENALTY = -0.3;
export const hookPostEditTool = {
    name: 'hook_post_edit',
    description: 'Post-edit learning: record outcome, update patterns, distill memories',
    parameters: z.object({
        filePath: z.string().describe('Path to file that was edited'),
        success: z.boolean().describe('Whether the edit was successful'),
        agent: z.string().optional().describe('Agent that performed the edit'),
        duration: z.number().optional().describe('Edit duration in ms'),
        errorMessage: z.string().optional().describe('Error message if failed')
    }),
    execute: async ({ filePath, success, agent, duration, errorMessage }, { onProgress }) => {
        const startTime = Date.now();
        const intel = loadIntelligence();
        const ext = path.extname(filePath);
        const state = `edit:${ext}`;
        // RUVECTOR INTELLIGENCE: Complete trajectory and learn
        let learningOutcome = null;
        let intelligenceEnabled = false;
        try {
            const trajectoryId = activeEditTrajectories.get(filePath);
            if (trajectoryId !== undefined) {
                // Record the final step
                await recordTrajectoryStep(trajectoryId, `edit-${success ? 'success' : 'failure'}`, success ? SUCCESS_REWARD : FAILURE_PENALTY, {
                    file: filePath,
                    errorFixed: success,
                    testPassed: success
                });
                // End trajectory and get learning outcome
                const quality = success ? 0.9 : 0.3;
                learningOutcome = await endTaskTrajectory(trajectoryId, success, quality);
                // Store successful pattern in ReasoningBank
                if (success && agent) {
                    await storePattern(`Edit ${ext} file: ${path.basename(filePath)}`, `Agent ${agent} successfully edited the file`, SUCCESS_REWARD);
                }
                // Clean up
                activeEditTrajectories.delete(filePath);
                intelligenceEnabled = true;
            }
        }
        catch (error) {
            console.debug('[PostEdit] Intelligence learning failed:', error);
        }
        // 1. Update Q-table pattern (fallback learning)
        if (agent) {
            if (!intel.patterns[state]) {
                intel.patterns[state] = {};
            }
            const currentValue = intel.patterns[state][agent] || 0;
            const reward = success ? SUCCESS_REWARD : FAILURE_PENALTY;
            // Q-learning update: Q(s,a) = Q(s,a) + α * (r - Q(s,a))
            intel.patterns[state][agent] = currentValue + LEARNING_RATE * (reward - currentValue);
        }
        // 2. Record in metrics
        intel.metrics.totalRoutes++;
        if (success) {
            intel.metrics.successfulRoutes++;
        }
        intel.metrics.routingHistory.push({
            timestamp: new Date().toISOString(),
            task: `edit:${filePath}`,
            agent: agent || 'unknown',
            success
        });
        // Keep last 100 entries
        if (intel.metrics.routingHistory.length > 100) {
            intel.metrics.routingHistory = intel.metrics.routingHistory.slice(-100);
        }
        // 3. Store error pattern if failed
        if (!success && errorMessage) {
            const existingPattern = intel.errorPatterns.find(p => p.errorType === errorMessage.split(':')[0]);
            if (existingPattern) {
                existingPattern.agentSuccess[agent || 'unknown'] =
                    (existingPattern.agentSuccess[agent || 'unknown'] || 0) - 1;
            }
            else {
                intel.errorPatterns.push({
                    errorType: errorMessage.split(':')[0] || 'UnknownError',
                    context: `${ext} file edit`,
                    resolution: '',
                    agentSuccess: { [agent || 'unknown']: -1 }
                });
            }
            // Keep last 50 error patterns
            if (intel.errorPatterns.length > 50) {
                intel.errorPatterns = intel.errorPatterns.slice(-50);
            }
        }
        // 4. Distill successful pattern as memory
        if (success && agent) {
            const memoryContent = `Successful ${ext} edit by ${agent} on ${path.basename(filePath)}`;
            intel.memories.push({
                content: memoryContent,
                type: 'success',
                created: new Date().toISOString(),
                embedding: simpleEmbed(memoryContent)
            });
            // Keep last 200 memories
            if (intel.memories.length > 200) {
                intel.memories = intel.memories.slice(-200);
            }
        }
        // 5. Save updated intelligence
        saveIntelligence(intel);
        const latency = Date.now() - startTime;
        // Get intelligence stats for monitoring
        let intelligenceStats = null;
        try {
            intelligenceStats = await getIntelligenceStats();
        }
        catch (e) {
            // Ignore if not available
        }
        return {
            success: true,
            patternsUpdated: true,
            newPatternValue: agent ? intel.patterns[state]?.[agent] : null,
            routingAccuracy: intel.metrics.totalRoutes > 0
                ? intel.metrics.successfulRoutes / intel.metrics.totalRoutes
                : 0,
            // RuVector Intelligence additions
            intelligenceEnabled,
            learningOutcome,
            intelligenceStats,
            latencyMs: latency,
            timestamp: new Date().toISOString()
        };
    }
};
//# sourceMappingURL=post-edit.js.map