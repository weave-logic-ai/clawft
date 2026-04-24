/**
 * P2P Swarm V2 Hooks Integration
 *
 * Provides hook handlers for P2P swarm coordination:
 * - PreToolUse: Check swarm connection, validate capabilities
 * - PostToolUse: Sync learning data to swarm
 * - SessionStart: Initialize swarm connection
 * - Stop: Cleanup swarm connection
 */
import { createP2PSwarmV2 } from '../swarm/p2p-swarm-v2.js';
import { logger } from '../utils/logger.js';
// Global swarm instance for hooks
let hooksSwarmInstance = null;
/**
 * Get or create swarm instance for hooks
 */
export async function getHooksSwarm(agentId, swarmKey) {
    if (!hooksSwarmInstance) {
        const id = agentId || `hooks-agent-${Date.now().toString(36)}`;
        hooksSwarmInstance = await createP2PSwarmV2(id, swarmKey);
        logger.info('P2P Swarm initialized for hooks', { agentId: id });
    }
    return hooksSwarmInstance;
}
/**
 * Disconnect hooks swarm
 */
export function disconnectHooksSwarm() {
    if (hooksSwarmInstance) {
        hooksSwarmInstance.disconnect();
        hooksSwarmInstance = null;
        logger.info('P2P Swarm disconnected from hooks');
    }
}
/**
 * SessionStart hook - Initialize P2P swarm connection
 */
export async function onSessionStart(config) {
    try {
        // Check if swarm is enabled
        if (process.env.AGENTIC_FLOW_P2P_SWARM !== 'true') {
            return {
                connected: false,
                agentId: '',
                swarmId: '',
                swarmKey: '',
                memberCount: 0,
            };
        }
        const swarm = await getHooksSwarm(config?.agentId || process.env.AGENTIC_FLOW_SWARM_AGENT_ID, config?.swarmKey || process.env.AGENTIC_FLOW_SWARM_KEY);
        if (config?.enableExecutor) {
            swarm.startTaskExecutor();
        }
        const status = swarm.getStatus();
        return {
            connected: status.connected,
            agentId: status.agentId,
            swarmId: status.swarmId,
            swarmKey: swarm.getSwarmKey(),
            memberCount: swarm.getLiveMemberCount(),
        };
    }
    catch (error) {
        logger.error('Failed to initialize P2P swarm on session start', { error });
        return {
            connected: false,
            agentId: '',
            swarmId: '',
            swarmKey: '',
            memberCount: 0,
        };
    }
}
/**
 * Stop hook - Cleanup P2P swarm connection
 */
export function onStop() {
    disconnectHooksSwarm();
}
/**
 * PreToolUse hook - Check swarm status before tool execution
 */
export async function onPreToolUse(toolName, params) {
    if (!hooksSwarmInstance) {
        return {
            allow: true, // Allow if swarm not initialized
            swarmConnected: false,
            liveMembers: 0,
        };
    }
    const status = hooksSwarmInstance.getStatus();
    const liveMembers = hooksSwarmInstance.getLiveMemberCount();
    // Provide recommendations based on swarm state
    let recommendation;
    if (toolName === 'Bash' || toolName === 'Edit' || toolName === 'Write') {
        // Suggest syncing after file operations
        recommendation = 'Consider syncing learning data to swarm after this operation';
    }
    return {
        allow: true,
        swarmConnected: status.connected,
        liveMembers,
        recommendation,
    };
}
/**
 * PostToolUse hook - Sync learning data after tool execution
 */
export async function onPostToolUse(toolName, params, result) {
    if (!hooksSwarmInstance) {
        return { synced: false };
    }
    try {
        // Sync relevant data based on tool type
        if (toolName === 'Edit' || toolName === 'Write') {
            // Publish file change notification
            const messageId = await hooksSwarmInstance.publish('file_changes', {
                tool: toolName,
                file: params.file_path || params.path,
                timestamp: Date.now(),
                success: !result?.error,
            });
            return {
                synced: true,
                syncType: 'file_change',
                messageId,
            };
        }
        if (toolName === 'Bash') {
            // Publish command execution notification
            const messageId = await hooksSwarmInstance.publish('commands', {
                command: (params.command || '').substring(0, 100),
                timestamp: Date.now(),
                success: result?.exitCode === 0,
            });
            return {
                synced: true,
                syncType: 'command',
                messageId,
            };
        }
        return { synced: false };
    }
    catch (error) {
        logger.warn('Failed to sync to P2P swarm', { error, toolName });
        return { synced: false };
    }
}
/**
 * Sync Q-table to swarm (for learning coordination)
 */
export async function syncQTable(qTable) {
    if (!hooksSwarmInstance) {
        return {
            success: false,
            error: 'P2P swarm not connected',
        };
    }
    try {
        const pointer = await hooksSwarmInstance.syncQTable(qTable);
        return {
            success: true,
            cid: pointer.cid,
        };
    }
    catch (error) {
        return {
            success: false,
            error: error instanceof Error ? error.message : String(error),
        };
    }
}
/**
 * Sync memory vectors to swarm
 */
export async function syncMemory(vectors, namespace = 'default') {
    if (!hooksSwarmInstance) {
        return {
            success: false,
            error: 'P2P swarm not connected',
        };
    }
    try {
        const pointer = await hooksSwarmInstance.syncMemory(vectors, namespace);
        return {
            success: true,
            cid: pointer.cid,
        };
    }
    catch (error) {
        return {
            success: false,
            error: error instanceof Error ? error.message : String(error),
        };
    }
}
/**
 * Get swarm status for hooks context
 */
export function getSwarmStatus() {
    if (!hooksSwarmInstance) {
        return {
            connected: false,
            liveMembers: 0,
        };
    }
    const status = hooksSwarmInstance.getStatus();
    return {
        connected: status.connected,
        agentId: status.agentId,
        swarmId: status.swarmId,
        liveMembers: hooksSwarmInstance.getLiveMemberCount(),
        relays: {
            healthy: status.relays.healthy,
            total: status.relays.total,
        },
    };
}
/**
 * Subscribe to swarm topic for real-time updates
 */
export function subscribeToTopic(topic, callback) {
    if (!hooksSwarmInstance) {
        logger.warn('Cannot subscribe: P2P swarm not connected');
        return;
    }
    hooksSwarmInstance.subscribe(topic, callback);
}
/**
 * Publish to swarm topic
 */
export async function publishToTopic(topic, payload) {
    if (!hooksSwarmInstance) {
        logger.warn('Cannot publish: P2P swarm not connected');
        return null;
    }
    try {
        return await hooksSwarmInstance.publish(topic, payload);
    }
    catch (error) {
        logger.error('Failed to publish to swarm', { error, topic });
        return null;
    }
}
// Export hook handlers for registration
export const p2pSwarmHooks = {
    onSessionStart,
    onStop,
    onPreToolUse,
    onPostToolUse,
    syncQTable,
    syncMemory,
    getSwarmStatus,
    subscribeToTopic,
    publishToTopic,
    getHooksSwarm,
    disconnectHooksSwarm,
};
export default p2pSwarmHooks;
//# sourceMappingURL=p2p-swarm-hooks.js.map