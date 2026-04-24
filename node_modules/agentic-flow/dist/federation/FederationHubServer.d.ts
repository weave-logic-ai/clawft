/**
 * Federation Hub Server - WebSocket-based hub for agent synchronization
 *
 * This is a production-ready implementation using WebSocket (HTTP/2 upgrade)
 * as a fallback until native QUIC is implemented.
 */
import { WebSocket } from 'ws';
export interface HubConfig {
    port?: number;
    dbPath?: string;
    maxAgents?: number;
    syncInterval?: number;
}
export interface AgentConnection {
    ws: WebSocket;
    agentId: string;
    tenantId: string;
    connectedAt: number;
    lastSyncAt: number;
    vectorClock: Record<string, number>;
}
export interface SyncMessage {
    type: 'auth' | 'pull' | 'push' | 'ack' | 'error';
    agentId?: string;
    tenantId?: string;
    token?: string;
    vectorClock?: Record<string, number>;
    data?: any[];
    error?: string;
    timestamp: number;
}
export declare class FederationHubServer {
    private config;
    private wss?;
    private server?;
    private connections;
    private db;
    private agentDB;
    private globalVectorClock;
    constructor(config: HubConfig);
    /**
     * Initialize hub database schema
     */
    private initializeDatabase;
    /**
     * Start the hub server
     */
    start(): Promise<void>;
    /**
     * Handle new agent connection
     */
    private handleConnection;
    /**
     * Handle authentication
     */
    private handleAuth;
    /**
     * Handle pull request (agent wants updates from hub)
     */
    private handlePull;
    /**
     * Handle push request (agent sending updates to hub)
     */
    private handlePush;
    /**
     * Get changes since a given vector clock
     * Returns memories from other agents in the same tenant
     */
    private getChangesSince;
    /**
     * Broadcast message to all agents in a tenant (except sender)
     */
    private broadcastToTenant;
    /**
     * Send message to WebSocket
     */
    private send;
    /**
     * Send error message
     */
    private sendError;
    /**
     * Get hub statistics
     */
    getStats(): {
        connectedAgents: number;
        totalEpisodes: number;
        tenants: number;
        uptime: number;
    };
    /**
     * Stop the hub server
     */
    stop(): Promise<void>;
    /**
     * Query patterns from AgentDB with tenant isolation
     */
    queryPatterns(tenantId: string, task: string, k?: number): Promise<any[]>;
}
//# sourceMappingURL=FederationHubServer.d.ts.map