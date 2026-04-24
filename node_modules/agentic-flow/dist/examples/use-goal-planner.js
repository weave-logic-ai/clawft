#!/usr/bin/env tsx
// Example: Using goal-planner agent from .claude/agents
import "dotenv/config";
import { getAgent } from "../utils/agentLoader.js";
import { claudeAgent } from "../agents/claudeAgent.js";
import { logger } from "../utils/logger.js";
async function main() {
    logger.setContext({ service: 'goal-planner-example', version: '1.0.0' });
    logger.info('Starting goal-planner example');
    // Load the goal-planner agent definition
    const goalPlanner = getAgent('goal-planner');
    if (!goalPlanner) {
        logger.error('goal-planner agent not found');
        console.error('❌ goal-planner agent not found in .claude/agents/');
        process.exit(1);
    }
    logger.info('Loaded goal-planner agent', {
        description: goalPlanner.description
    });
    // Use the goal-planner for a complex task
    const task = `
Research the Claude Agent SDK and create a comprehensive improvement plan for our Docker implementation.

Current state:
- Basic parallel agent execution
- Limited error handling
- No observability

Goals:
1. Identify SDK capabilities we're not using
2. Propose specific improvements
3. Create actionable implementation plan
4. Estimate effort and ROI
  `.trim();
    console.log('\n🎯 Task:', task.substring(0, 100) + '...\n');
    console.log('⏳ Running goal-planner agent...\n');
    const result = await claudeAgent(goalPlanner, task, (chunk) => {
        if (process.env.ENABLE_STREAMING === 'true') {
            process.stdout.write(chunk);
        }
    });
    console.log('\n✅ Goal-planner completed!\n');
    console.log('═══════════════════════════════════════\n');
    console.log(result.output);
    console.log('\n═══════════════════════════════════════\n');
    logger.info('Example completed successfully');
}
main().catch(err => {
    logger.error('Example failed', { error: err });
    console.error('❌ Error:', err.message);
    process.exit(1);
});
//# sourceMappingURL=use-goal-planner.js.map