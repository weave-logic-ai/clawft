/**
 * Federation Hub - QUIC-based synchronization hub for ephemeral agents
 *
 * Features:
 * - QUIC protocol for low-latency sync (<50ms)
 * - mTLS for transport security
 * - Vector clocks for conflict resolution
 * - Hub-and-spoke topology support
 */
type AgentDB = any;
export interface FederationHubConfig {
    endpoint: string;
    agentId: string;
    tenantId: string;
    token: string;
    enableMTLS?: boolean;
    certPath?: string;
    keyPath?: string;
    caPath?: string;
}
export interface SyncMessage {
    type: 'push' | 'pull' | 'ack';
    agentId: string;
    tenantId: string;
    vectorClock: Record<string, number>;
    data?: any[];
    timestamp: number;
}
export declare class FederationHub {
    private config;
    private connected;
    private vectorClock;
    private lastSyncTime;
    constructor(config: FederationHubConfig);
    /**
     * Connect to federation hub with mTLS
     */
    connect(): Promise<void>;
    /**
     * Synchronize local database with federation hub
     *
     * 1. Pull: Get updates from hub (other agents' changes)
     * 2. Push: Send local changes to hub
     * 3. Resolve conflicts using vector clocks
     */
    sync(db: AgentDB): Promise<void>;
    /**
     * Send sync message to hub via QUIC
     */
    private sendSyncMessage;
    /**
     * Get local changes since last sync
     */
    private getLocalChanges;
    /**
     * Merge remote updates into local database
     * Uses vector clocks to detect and resolve conflicts
     */
    private mergeRemoteUpdates;
    /**
     * Detect conflicts using vector clocks
     */
    private detectConflict;
    /**
     * Update local vector clock with remote timestamps
     */
    private updateVectorClock;
    /**
     * Apply update to local database
     */
    private applyUpdate;
    /**
     * Disconnect from federation hub
     */
    disconnect(): Promise<void>;
    /**
     * Get connection status
     */
    isConnected(): boolean;
    /**
     * Get sync statistics
     */
    getSyncStats(): {
        lastSyncTime: number;
        vectorClock: Record<string, number>;
    };
}
export {};
//# sourceMappingURL=FederationHub.d.ts.map