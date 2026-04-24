// Agent with Claude Flow memory and coordination capabilities
import { query } from '@anthropic-ai/claude-agent-sdk';
import { logger } from '../utils/logger.js';
import { withRetry } from '../utils/retry.js';
import { toolConfig } from '../config/tools.js';
import { getMemoryConfig, getSwarmConfig } from '../config/claudeFlow.js';
/**
 * Execute agent with Claude Flow memory and coordination
 */
export async function claudeFlowAgent(agentName, systemPrompt, input, options = {}) {
    const { enableMemory = true, enableCoordination = false, memoryNamespace, swarmTopology = 'mesh', onStream } = options;
    const startTime = Date.now();
    logger.info('Starting Claude Flow agent', {
        agentName,
        input: input.substring(0, 100),
        enableMemory,
        enableCoordination
    });
    // Prepare memory context if enabled
    let memoryContext = '';
    if (enableMemory) {
        const memoryConfig = getMemoryConfig(memoryNamespace || agentName);
        memoryContext = `

You have access to persistent memory via claude-flow tools:
- Store: mcp__claude-flow__memory_usage with action="store", namespace="${memoryConfig.namespace}"
- Retrieve: mcp__claude-flow__memory_usage with action="retrieve", namespace="${memoryConfig.namespace}"
- Search: mcp__claude-flow__memory_search with pattern="search query"

Use memory to persist important information across conversations.`;
    }
    // Prepare coordination context if enabled
    let coordinationContext = '';
    if (enableCoordination) {
        const swarmConfig = getSwarmConfig(swarmTopology);
        coordinationContext = `

You have access to swarm coordination via claude-flow tools:
- Init swarm: mcp__claude-flow__swarm_init with topology="${swarmConfig.topology}"
- Spawn agent: mcp__claude-flow__agent_spawn with type="researcher|coder|analyst"
- Orchestrate: mcp__claude-flow__task_orchestrate with task="description"
- Check status: mcp__claude-flow__swarm_status

Use coordination for complex multi-agent tasks.`;
    }
    const enhancedSystemPrompt = `${systemPrompt}${memoryContext}${coordinationContext}`;
    return withRetry(async () => {
        const result = query({
            prompt: input,
            options: {
                systemPrompt: enhancedSystemPrompt,
                ...toolConfig
            }
        });
        let output = '';
        for await (const msg of result) {
            if (msg.type === 'assistant') {
                const chunk = msg.message.content
                    ?.map((c) => (c.type === 'text' ? c.text : ''))
                    .join('') || '';
                output += chunk;
                if (onStream && chunk) {
                    onStream(chunk);
                }
            }
        }
        const duration = Date.now() - startTime;
        logger.info('Claude Flow agent completed', {
            agentName,
            duration,
            outputLength: output.length,
            memoryUsed: enableMemory,
            coordinationUsed: enableCoordination
        });
        return { output };
    });
}
/**
 * Example: Memory-enabled research agent
 */
export async function memoryResearchAgent(topic, onStream) {
    return claudeFlowAgent('memory-researcher', `You are a research agent with persistent memory.

Research the given topic thoroughly and store key findings in memory for future reference.
Use memory_usage tool to store important facts, insights, and references.`, `Research topic: ${topic}`, {
        enableMemory: true,
        enableCoordination: false,
        onStream
    });
}
/**
 * Example: Coordination-enabled orchestrator agent
 */
export async function orchestratorAgent(task, onStream) {
    return claudeFlowAgent('orchestrator', `You are an orchestration agent that coordinates multiple specialized agents.

Break down complex tasks and delegate to specialized agents using swarm coordination.
Use swarm_init, agent_spawn, and task_orchestrate tools to manage the workflow.`, `Orchestrate task: ${task}`, {
        enableMemory: true,
        enableCoordination: true,
        swarmTopology: 'hierarchical',
        onStream
    });
}
/**
 * Example: Full-featured agent with memory and coordination
 */
export async function hybridAgent(task, agentName = 'hybrid', systemPrompt = 'You are a versatile agent with memory and coordination capabilities.', onStream) {
    return claudeFlowAgent(agentName, systemPrompt, task, {
        enableMemory: true,
        enableCoordination: true,
        swarmTopology: 'mesh',
        onStream
    });
}
//# sourceMappingURL=claudeFlowAgent.js.map