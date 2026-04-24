#!/usr/bin/env tsx
// Example: Multi-agent orchestration using .claude/agents
import "dotenv/config";
import { loadAgents } from "../utils/agentLoader.js";
import { claudeAgent } from "../agents/claudeAgent.js";
import { logger } from "../utils/logger.js";
async function main() {
    logger.setContext({ service: 'multi-agent-orchestration', version: '1.0.0' });
    logger.info('Starting multi-agent orchestration example');
    // Load all available agents
    const agents = loadAgents();
    console.log(`\n📦 Loaded ${agents.size} agents from .claude/agents/\n`);
    // List some interesting agents
    const agentNames = Array.from(agents.keys());
    console.log('Available agents:');
    agentNames.slice(0, 10).forEach(name => {
        const agent = agents.get(name);
        console.log(`  • ${name}: ${agent.description.substring(0, 80)}...`);
    });
    console.log(`  ... and ${agentNames.length - 10} more\n`);
    // Example: Use multiple agents in sequence
    console.log('🔄 Running multi-agent workflow...\n');
    // 1. Use goal-planner to create a plan
    const goalPlanner = agents.get('goal-planner');
    if (goalPlanner) {
        console.log('Step 1: Using goal-planner to create improvement plan');
        const planResult = await claudeAgent(goalPlanner, 'Create a 3-step plan to improve our Claude Agent SDK implementation with error handling, logging, and monitoring.');
        console.log('\n✅ Plan created\n');
        console.log(planResult.output.substring(0, 300) + '...\n');
    }
    // 2. Use code-analyzer for implementation review
    const codeAnalyzer = agents.get('code-analyzer');
    if (codeAnalyzer) {
        console.log('Step 2: Using code-analyzer to review current implementation');
        const analysisResult = await claudeAgent(codeAnalyzer, 'Analyze the code quality of a Claude Agent SDK implementation focusing on error handling patterns and logging.');
        console.log('\n✅ Analysis complete\n');
        console.log(analysisResult.output.substring(0, 300) + '...\n');
    }
    logger.info('Multi-agent orchestration completed successfully');
}
main().catch(err => {
    logger.error('Multi-agent orchestration failed', { error: err });
    console.error('❌ Error:', err.message);
    process.exit(1);
});
//# sourceMappingURL=multi-agent-orchestration.js.map