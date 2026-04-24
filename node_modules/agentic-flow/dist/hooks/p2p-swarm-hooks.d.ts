/**
 * P2P Swarm V2 Hooks Integration
 *
 * Provides hook handlers for P2P swarm coordination:
 * - PreToolUse: Check swarm connection, validate capabilities
 * - PostToolUse: Sync learning data to swarm
 * - SessionStart: Initialize swarm connection
 * - Stop: Cleanup swarm connection
 */
import { P2PSwarmV2 } from '../swarm/p2p-swarm-v2.js';
/**
 * Get or create swarm instance for hooks
 */
export declare function getHooksSwarm(agentId?: string, swarmKey?: string): Promise<P2PSwarmV2>;
/**
 * Disconnect hooks swarm
 */
export declare function disconnectHooksSwarm(): void;
/**
 * SessionStart hook - Initialize P2P swarm connection
 */
export declare function onSessionStart(config?: {
    agentId?: string;
    swarmKey?: string;
    enableExecutor?: boolean;
}): Promise<{
    connected: boolean;
    agentId: string;
    swarmId: string;
    swarmKey: string;
    memberCount: number;
}>;
/**
 * Stop hook - Cleanup P2P swarm connection
 */
export declare function onStop(): void;
/**
 * PreToolUse hook - Check swarm status before tool execution
 */
export declare function onPreToolUse(toolName: string, params: Record<string, any>): Promise<{
    allow: boolean;
    swarmConnected: boolean;
    liveMembers: number;
    recommendation?: string;
}>;
/**
 * PostToolUse hook - Sync learning data after tool execution
 */
export declare function onPostToolUse(toolName: string, params: Record<string, any>, result: any): Promise<{
    synced: boolean;
    syncType?: string;
    messageId?: string;
}>;
/**
 * Sync Q-table to swarm (for learning coordination)
 */
export declare function syncQTable(qTable: number[][]): Promise<{
    success: boolean;
    cid?: string;
    error?: string;
}>;
/**
 * Sync memory vectors to swarm
 */
export declare function syncMemory(vectors: number[][], namespace?: string): Promise<{
    success: boolean;
    cid?: string;
    error?: string;
}>;
/**
 * Get swarm status for hooks context
 */
export declare function getSwarmStatus(): {
    connected: boolean;
    agentId?: string;
    swarmId?: string;
    liveMembers: number;
    relays?: {
        healthy: number;
        total: number;
    };
};
/**
 * Subscribe to swarm topic for real-time updates
 */
export declare function subscribeToTopic(topic: string, callback: (data: any, from: string) => void): void;
/**
 * Publish to swarm topic
 */
export declare function publishToTopic(topic: string, payload: any): Promise<string | null>;
export declare const p2pSwarmHooks: {
    onSessionStart: typeof onSessionStart;
    onStop: typeof onStop;
    onPreToolUse: typeof onPreToolUse;
    onPostToolUse: typeof onPostToolUse;
    syncQTable: typeof syncQTable;
    syncMemory: typeof syncMemory;
    getSwarmStatus: typeof getSwarmStatus;
    subscribeToTopic: typeof subscribeToTopic;
    publishToTopic: typeof publishToTopic;
    getHooksSwarm: typeof getHooksSwarm;
    disconnectHooksSwarm: typeof disconnectHooksSwarm;
};
export default p2pSwarmHooks;
//# sourceMappingURL=p2p-swarm-hooks.d.ts.map