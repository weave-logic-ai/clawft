#!/usr/bin/env node
/**
 * Pre-Task Hook for ReasoningBank
 * Retrieves and injects relevant memories before task execution
 *
 * Usage: tsx hooks/pre-task.ts --task-id <id> --query <query> [--domain <domain>] [--agent <agent>]
 */
import { retrieveMemories, formatMemoriesForPrompt } from '../core/retrieve.js';
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
        else if (key === 'query')
            parsed.query = value;
        else if (key === 'domain')
            parsed.domain = value;
        else if (key === 'agent')
            parsed.agent = value;
    }
    if (!parsed.taskId || !parsed.query) {
        console.error('Usage: pre-task.ts --task-id <id> --query <query> [--domain <domain>] [--agent <agent>]');
        process.exit(1);
    }
    return parsed;
}
async function main() {
    const config = loadConfig();
    // Check if ReasoningBank is enabled
    if (!config.features?.enable_pre_task_hook) {
        console.log('[INFO] ReasoningBank pre-task hook is disabled');
        process.exit(0);
    }
    const args = parseArgs();
    console.log(`[PRE-TASK] Task ID: ${args.taskId}`);
    console.log(`[PRE-TASK] Query: ${args.query}`);
    try {
        // Retrieve relevant memories
        const memories = await retrieveMemories(args.query, {
            domain: args.domain,
            agent: args.agent,
            k: config.retrieve.k
        });
        if (memories.length === 0) {
            console.log('[PRE-TASK] No relevant memories found');
            process.exit(0);
        }
        console.log(`[PRE-TASK] Retrieved ${memories.length} relevant memories`);
        // Format for injection into system prompt
        const formattedMemories = formatMemoriesForPrompt(memories);
        // Output to stdout for injection
        // The agent system should capture this and inject into the system prompt
        console.log('\n=== MEMORIES FOR INJECTION ===');
        console.log(formattedMemories);
        console.log('=== END MEMORIES ===\n');
        process.exit(0);
    }
    catch (error) {
        console.error('[PRE-TASK ERROR]', error);
        process.exit(1);
    }
}
main();
//# sourceMappingURL=pre-task.js.map