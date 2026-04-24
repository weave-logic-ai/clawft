/**
 * Agent Converter - Converts agentic-flow agent definitions to Claude Agent SDK format
 *
 * Takes agents defined in .claude/agents/ and converts them to SDK's agents option format
 * for native subagent support via the Task tool.
 */
import { logger } from "../utils/logger.js";
import { listAgents, getAgent } from "../utils/agentLoader.js";
/**
 * Default tools for different agent types
 */
const AGENT_TYPE_TOOLS = {
    'researcher': ['Read', 'Glob', 'Grep', 'WebSearch', 'WebFetch'],
    'coder': ['Read', 'Write', 'Edit', 'Bash', 'Glob', 'Grep'],
    'analyst': ['Read', 'Glob', 'Grep', 'WebSearch', 'WebFetch'],
    'reviewer': ['Read', 'Glob', 'Grep'],
    'tester': ['Read', 'Glob', 'Grep', 'Bash'],
    'documenter': ['Read', 'Write', 'Edit', 'Glob', 'Grep'],
    'architect': ['Read', 'Glob', 'Grep', 'WebSearch'],
    'coordinator': ['Read', 'Glob', 'Grep', 'Task'], // Can spawn subagents
    'default': ['Read', 'Glob', 'Grep']
};
/**
 * Infer agent type from name or description
 */
function inferAgentType(agent) {
    const name = agent.name.toLowerCase();
    const desc = agent.description.toLowerCase();
    if (name.includes('research') || desc.includes('research'))
        return 'researcher';
    if (name.includes('code') || desc.includes('code') || desc.includes('implement'))
        return 'coder';
    if (name.includes('review') || desc.includes('review'))
        return 'reviewer';
    if (name.includes('test') || desc.includes('test'))
        return 'tester';
    if (name.includes('doc') || desc.includes('document'))
        return 'documenter';
    if (name.includes('arch') || desc.includes('architect'))
        return 'architect';
    if (name.includes('coord') || desc.includes('orchestrat'))
        return 'coordinator';
    if (name.includes('analy') || desc.includes('analy'))
        return 'analyst';
    return 'default';
}
/**
 * Infer model for agent based on complexity
 */
function inferAgentModel(agent) {
    const name = agent.name.toLowerCase();
    const desc = agent.description.toLowerCase();
    // Use opus for complex tasks
    if (desc.includes('complex') || desc.includes('architect') || desc.includes('security')) {
        return 'opus';
    }
    // Use haiku for simple/fast tasks
    if (name.includes('format') || name.includes('simple') || desc.includes('quick')) {
        return 'haiku';
    }
    // Default to inherit (use parent model)
    return 'inherit';
}
/**
 * Convert a single agentic-flow agent to SDK format
 */
export function convertAgentToSdkFormat(agent) {
    const agentType = inferAgentType(agent);
    const tools = AGENT_TYPE_TOOLS[agentType] || AGENT_TYPE_TOOLS.default;
    return {
        description: agent.description,
        prompt: agent.systemPrompt,
        tools,
        model: inferAgentModel(agent)
    };
}
// Cache for converted agents (invalidated on reload)
let cachedSdkAgents = null;
let cacheTimestamp = 0;
const CACHE_TTL_MS = 60 * 1000; // 1 minute cache
/**
 * Convert all loaded agents to SDK format (with caching)
 */
export function convertAllAgentsToSdkFormat() {
    // Return cached if valid
    if (cachedSdkAgents && Date.now() - cacheTimestamp < CACHE_TTL_MS) {
        return cachedSdkAgents;
    }
    const sdkAgents = {};
    try {
        const agents = listAgents();
        for (const agentInfo of agents) {
            try {
                const agent = getAgent(agentInfo.name);
                if (agent) {
                    // Use sanitized name (replace spaces/special chars)
                    const safeName = agentInfo.name.replace(/[^a-zA-Z0-9-_]/g, '-').toLowerCase();
                    sdkAgents[safeName] = convertAgentToSdkFormat(agent);
                }
            }
            catch (error) {
                logger.warn('Failed to convert agent', { name: agentInfo.name, error: error.message });
            }
        }
        logger.info('Converted agents to SDK format', { count: Object.keys(sdkAgents).length });
    }
    catch (error) {
        logger.warn('Failed to list agents', { error: error.message });
    }
    // Update cache
    cachedSdkAgents = sdkAgents;
    cacheTimestamp = Date.now();
    return sdkAgents;
}
/**
 * Invalidate agent cache (call after agent definitions change)
 */
export function invalidateAgentCache() {
    cachedSdkAgents = null;
    cacheTimestamp = 0;
}
/**
 * Get essential agents for most tasks
 */
export function getEssentialAgents() {
    return {
        'researcher': {
            description: 'Expert at researching codebases, finding patterns, and gathering context',
            prompt: 'You are a research specialist. Find relevant code, patterns, and documentation. Provide comprehensive context for the task.',
            tools: ['Read', 'Glob', 'Grep', 'WebSearch', 'WebFetch'],
            model: 'sonnet'
        },
        'coder': {
            description: 'Expert at implementing code changes, writing new features, and fixing bugs',
            prompt: 'You are a coding specialist. Write clean, well-tested code. Follow existing patterns and conventions.',
            tools: ['Read', 'Write', 'Edit', 'Bash', 'Glob', 'Grep'],
            model: 'sonnet'
        },
        'reviewer': {
            description: 'Expert at code review, finding issues, and suggesting improvements',
            prompt: 'You are a code review specialist. Find bugs, security issues, and code quality problems. Suggest specific improvements.',
            tools: ['Read', 'Glob', 'Grep'],
            model: 'sonnet'
        },
        'tester': {
            description: 'Expert at writing and running tests, ensuring code quality',
            prompt: 'You are a testing specialist. Write comprehensive tests. Verify code works correctly.',
            tools: ['Read', 'Glob', 'Grep', 'Bash'],
            model: 'sonnet'
        },
        'documenter': {
            description: 'Expert at writing documentation, comments, and explanations',
            prompt: 'You are a documentation specialist. Write clear, comprehensive documentation. Update READMEs and comments.',
            tools: ['Read', 'Write', 'Edit', 'Glob', 'Grep'],
            model: 'haiku'
        }
    };
}
/**
 * Get agents for a specific use case
 */
export function getAgentsForUseCase(useCase) {
    const essential = getEssentialAgents();
    switch (useCase) {
        case 'code':
            return { coder: essential.coder, tester: essential.tester };
        case 'research':
            return { researcher: essential.researcher };
        case 'review':
            return { reviewer: essential.reviewer };
        case 'full':
        default:
            return essential;
    }
}
/**
 * Merge custom agents with essential agents
 */
export function getMergedAgents(includeCustom = true) {
    const essential = getEssentialAgents();
    if (!includeCustom) {
        return essential;
    }
    const custom = convertAllAgentsToSdkFormat();
    // Merge with custom taking precedence
    return { ...essential, ...custom };
}
//# sourceMappingURL=agent-converter.js.map