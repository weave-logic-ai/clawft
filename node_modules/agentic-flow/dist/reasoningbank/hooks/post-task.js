#!/usr/bin/env node
/**
 * Post-Task Hook for ReasoningBank
 * Judges trajectory, distills memories, and runs consolidation
 *
 * Usage: tsx hooks/post-task.ts --task-id <id> [--trajectory-file <file>]
 */
import { readFileSync } from 'fs';
import { judgeTrajectory } from '../core/judge.js';
import { distillMemories } from '../core/distill.js';
import { consolidate, shouldConsolidate } from '../core/consolidate.js';
import { loadConfig } from '../utils/config.js';
// Parse command line arguments
function parseArgs() {
    const args = process.argv.slice(2);
    const parsed = {};
    for (let i = 0; i < args.length; i += 2) {
        const key = args[i].replace(/^--/, '');
        const value = args[i + 1];
        if (key === 'task-id')
            parsed.taskId = value;
        else if (key === 'trajectory-file')
            parsed.trajectoryFile = value;
    }
    if (!parsed.taskId) {
        console.error('Usage: post-task.ts --task-id <id> [--trajectory-file <file>]');
        process.exit(1);
    }
    return parsed;
}
// Load trajectory from file or stdin
function loadTrajectory(filePath) {
    let content;
    if (filePath) {
        content = readFileSync(filePath, 'utf-8');
    }
    else {
        // Read from stdin (for piped input)
        content = readFileSync(0, 'utf-8');
    }
    try {
        const data = JSON.parse(content);
        return {
            trajectory: {
                steps: data.steps || data.trajectory?.steps || [],
                metadata: data.metadata || {}
            },
            query: data.query || data.task_query || 'Unknown task'
        };
    }
    catch (error) {
        console.error('[POST-TASK ERROR] Failed to parse trajectory JSON:', error);
        process.exit(1);
    }
}
async function main() {
    const config = loadConfig();
    // Check if ReasoningBank is enabled
    if (!config.features?.enable_post_task_hook) {
        console.log('[INFO] ReasoningBank post-task hook is disabled');
        process.exit(0);
    }
    const args = parseArgs();
    console.log(`[POST-TASK] Task ID: ${args.taskId}`);
    try {
        // Load trajectory
        const { trajectory, query } = loadTrajectory(args.trajectoryFile);
        console.log(`[POST-TASK] Query: ${query}`);
        console.log(`[POST-TASK] Trajectory steps: ${trajectory.steps.length}`);
        // Step 1: Judge trajectory
        console.log('[POST-TASK] Judging trajectory...');
        const verdict = await judgeTrajectory(trajectory, query);
        console.log(`[POST-TASK] Verdict: ${verdict.label} (confidence: ${verdict.confidence})`);
        console.log(`[POST-TASK] Reasons: ${verdict.reasons.join(', ')}`);
        // Step 2: Distill memories
        console.log('[POST-TASK] Distilling memories...');
        const memoryIds = await distillMemories(trajectory, verdict, query, {
            taskId: args.taskId,
            agentId: 'unknown', // Could be passed as argument
            domain: undefined // Could be extracted from query/trajectory
        });
        console.log(`[POST-TASK] Distilled ${memoryIds.length} memories`);
        // Step 3: Check if consolidation should run
        if (shouldConsolidate()) {
            console.log('[POST-TASK] Consolidation threshold reached, running consolidation...');
            const result = await consolidate();
            console.log(`[POST-TASK] Consolidation complete:`);
            console.log(`  - Processed: ${result.itemsProcessed} memories`);
            console.log(`  - Duplicates: ${result.duplicatesFound}`);
            console.log(`  - Contradictions: ${result.contradictionsFound}`);
            console.log(`  - Pruned: ${result.itemsPruned}`);
            console.log(`  - Duration: ${result.durationMs}ms`);
        }
        else {
            console.log('[POST-TASK] Consolidation threshold not reached, skipping');
        }
        // Output summary
        console.log('\n=== POST-TASK SUMMARY ===');
        console.log(`Verdict: ${verdict.label}`);
        console.log(`Memories distilled: ${memoryIds.length}`);
        console.log('=== END SUMMARY ===\n');
        process.exit(0);
    }
    catch (error) {
        console.error('[POST-TASK ERROR]', error);
        process.exit(1);
    }
}
main();
//# sourceMappingURL=post-task.js.map