/**
 * Federation Hub Client - WebSocket client for agent-to-hub communication
 */
type AgentDB = any;
export interface HubClientConfig {
    endpoint: string;
    agentId: string;
    tenantId: string;
    token: string;
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
export declare class FederationHubClient {
    private config;
    private ws?;
    private connected;
    private vectorClock;
    private lastSyncTime;
    private messageHandlers;
    constructor(config: HubClientConfig);
    /**
     * Connect to hub with WebSocket
     */
    connect(): Promise<void>;
    /**
     * Handle incoming message
     */
    private handleMessage;
    /**
     * Sync with hub
     */
    sync(db: AgentDB): Promise<void>;
    /**
     * Get local changes from database
     */
    private getLocalChanges;
    /**
     * Update vector clock
     */
    private updateVectorClock;
    /**
     * Send message to hub
     */
    private send;
    /**
     * Disconnect from hub
     */
    disconnect(): Promise<void>;
    /**
     * Check connection status
     */
    isConnected(): boolean;
    /**
     * Get sync stats
     */
    getSyncStats(): {
        lastSyncTime: number;
        vectorClock: Record<string, number>;
    };
}
export {};
//# sourceMappingURL=FederationHubClient.d.ts.map