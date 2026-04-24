/**
 * SDK Hooks Bridge - Connects agentic-flow intelligence layer to Claude Agent SDK hooks
 *
 * Bridges our custom hooks (intelligence-bridge.ts) with the native SDK hook system
 * enabling seamless integration with Claude Code's event loop.
 */
import { logger } from "../utils/logger.js";
// Lazy import intelligence bridge to avoid circular dependencies
let intelligenceBridge = null;
async function getIntelligenceBridge() {
    if (!intelligenceBridge) {
        try {
            intelligenceBridge = await import("../mcp/fastmcp/tools/hooks/intelligence-bridge.js");
        }
        catch (e) {
            logger.warn('Intelligence bridge not available', { error: e.message });
            return null;
        }
    }
    return intelligenceBridge;
}
// Active trajectory tracking with TTL (5 minutes max)
const TRAJECTORY_TTL_MS = 5 * 60 * 1000;
const activeTrajectories = new Map();
// Cleanup stale trajectories periodically
function cleanupStaleTrajectories() {
    const now = Date.now();
    for (const [key, value] of activeTrajectories.entries()) {
        if (now - value.timestamp > TRAJECTORY_TTL_MS) {
            activeTrajectories.delete(key);
        }
    }
}
// Run cleanup every 2 minutes
setInterval(cleanupStaleTrajectories, 2 * 60 * 1000).unref();
/**
 * PreToolUse hook - Called before tool execution
 * Routes to best agent and starts trajectory tracking
 */
export const preToolUseHook = async (input, toolUseId, { signal }) => {
    if (input.hook_event_name !== 'PreToolUse')
        return {};
    const { tool_name, tool_input, session_id } = input;
    try {
        const bridge = await getIntelligenceBridge();
        if (!bridge)
            return {};
        // Start trajectory for edit operations
        if (['Edit', 'Write', 'Bash'].includes(tool_name)) {
            const filePath = tool_input?.file_path || tool_input?.command || 'unknown';
            const result = await bridge.beginTaskTrajectory(`${tool_name}: ${filePath.substring(0, 100)}`, 'coder');
            if (result.success && result.trajectoryId > 0) {
                activeTrajectories.set(`${session_id}:${toolUseId}`, {
                    trajectoryId: result.trajectoryId,
                    timestamp: Date.now()
                });
                logger.debug('Trajectory started', { trajectoryId: result.trajectoryId, tool: tool_name });
            }
        }
        return {};
    }
    catch (error) {
        logger.warn('PreToolUse hook error', { error: error.message });
        return {};
    }
};
/**
 * PostToolUse hook - Called after successful tool execution
 * Records patterns and ends trajectories
 */
export const postToolUseHook = async (input, toolUseId, { signal }) => {
    if (input.hook_event_name !== 'PostToolUse')
        return {};
    const { tool_name, tool_input, tool_response, session_id } = input;
    try {
        const bridge = await getIntelligenceBridge();
        if (!bridge)
            return {};
        // End trajectory if one was started
        const trajectoryKey = `${session_id}:${toolUseId}`;
        const trajectoryEntry = activeTrajectories.get(trajectoryKey);
        if (trajectoryEntry) {
            await bridge.endTaskTrajectory(trajectoryEntry.trajectoryId, 'success');
            activeTrajectories.delete(trajectoryKey);
            logger.debug('Trajectory completed', { trajectoryId: trajectoryEntry.trajectoryId, tool: tool_name });
        }
        // Store successful pattern
        if (['Edit', 'Write'].includes(tool_name)) {
            const filePath = tool_input?.file_path || 'unknown';
            await bridge.storePattern({
                id: `sdk-${tool_name.toLowerCase()}-${Date.now()}`,
                metadata: {
                    tool: tool_name,
                    file: filePath,
                    success: true,
                    timestamp: Date.now()
                }
            });
        }
        return {};
    }
    catch (error) {
        logger.warn('PostToolUse hook error', { error: error.message });
        return {};
    }
};
/**
 * PostToolUseFailure hook - Called when tool execution fails
 * Ends trajectories as failures
 */
export const postToolUseFailureHook = async (input, toolUseId, { signal }) => {
    if (input.hook_event_name !== 'PostToolUseFailure')
        return {};
    const { session_id } = input;
    try {
        const bridge = await getIntelligenceBridge();
        if (!bridge)
            return {};
        // End trajectory as failure
        const trajectoryKey = `${session_id}:${toolUseId}`;
        const trajectoryEntry = activeTrajectories.get(trajectoryKey);
        if (trajectoryEntry) {
            await bridge.endTaskTrajectory(trajectoryEntry.trajectoryId, 'failure');
            activeTrajectories.delete(trajectoryKey);
            logger.debug('Trajectory failed', { trajectoryId: trajectoryEntry.trajectoryId });
        }
        return {};
    }
    catch (error) {
        logger.warn('PostToolUseFailure hook error', { error: error.message });
        return {};
    }
};
/**
 * SessionStart hook - Called when session begins
 * Initializes intelligence layer
 */
export const sessionStartHook = async (input, toolUseId, { signal }) => {
    if (input.hook_event_name !== 'SessionStart')
        return {};
    const { source, session_id } = input;
    try {
        const bridge = await getIntelligenceBridge();
        if (!bridge) {
            return {
                hookSpecificOutput: {
                    hookEventName: 'SessionStart',
                    additionalContext: 'Intelligence layer not available.'
                }
            };
        }
        const stats = await bridge.getIntelligenceStats();
        const message = `RuVector Intelligence active. ` +
            `Trajectories: ${stats.trajectoryCount}, ` +
            `Features: ${stats.features?.join(', ') || 'none'}`;
        logger.info('Session started', { sessionId: session_id, source, stats });
        return {
            hookSpecificOutput: {
                hookEventName: 'SessionStart',
                additionalContext: message
            }
        };
    }
    catch (error) {
        logger.warn('SessionStart hook error', { error: error.message });
        return {};
    }
};
/**
 * SessionEnd hook - Called when session ends
 * Persists learning data
 */
export const sessionEndHook = async (input, toolUseId, { signal }) => {
    if (input.hook_event_name !== 'SessionEnd')
        return {};
    const { reason, session_id } = input;
    try {
        const bridge = await getIntelligenceBridge();
        if (!bridge)
            return {};
        // Force learning cycle on session end
        await bridge.forceLearningCycle();
        logger.info('Session ended', { sessionId: session_id, reason });
        return {};
    }
    catch (error) {
        logger.warn('SessionEnd hook error', { error: error.message });
        return {};
    }
};
/**
 * SubagentStart hook - Called when a subagent is spawned
 */
export const subagentStartHook = async (input, toolUseId, { signal }) => {
    if (input.hook_event_name !== 'SubagentStart')
        return {};
    const { agent_id, agent_type } = input;
    logger.info('Subagent started', { agentId: agent_id, agentType: agent_type });
    return {};
};
/**
 * SubagentStop hook - Called when a subagent completes
 */
export const subagentStopHook = async (input, toolUseId, { signal }) => {
    if (input.hook_event_name !== 'SubagentStop')
        return {};
    logger.info('Subagent stopped');
    return {};
};
/**
 * Get SDK hooks configuration
 * Returns hooks in the format expected by Claude Agent SDK query() options
 */
export function getSdkHooks() {
    return {
        PreToolUse: [{ hooks: [preToolUseHook] }],
        PostToolUse: [{ hooks: [postToolUseHook] }],
        PostToolUseFailure: [{ hooks: [postToolUseFailureHook] }],
        SessionStart: [{ hooks: [sessionStartHook] }],
        SessionEnd: [{ hooks: [sessionEndHook] }],
        SubagentStart: [{ hooks: [subagentStartHook] }],
        SubagentStop: [{ hooks: [subagentStopHook] }]
    };
}
/**
 * Get filtered hooks for specific tools
 */
export function getToolSpecificHooks(toolMatcher) {
    return {
        PreToolUse: [{ matcher: toolMatcher, hooks: [preToolUseHook] }],
        PostToolUse: [{ matcher: toolMatcher, hooks: [postToolUseHook] }],
        PostToolUseFailure: [{ matcher: toolMatcher, hooks: [postToolUseFailureHook] }]
    };
}
//# sourceMappingURL=hooks-bridge.js.map