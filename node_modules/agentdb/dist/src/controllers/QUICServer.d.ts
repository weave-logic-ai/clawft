/**
 * QUICServer - QUIC Protocol Server for AgentDB Synchronization
 *
 * Implements a QUIC server for receiving and handling synchronization requests
 * from remote AgentDB instances. Supports episodes, skills, and edge synchronization.
 *
 * Features:
 * - Start/stop server lifecycle management
 * - Client connection handling
 * - Authentication and authorization
 * - Rate limiting per client
 * - Sync request processing (episodes, skills, edges)
 * - Comprehensive error handling and logging
 */
type Database = any;
export interface QUICServerConfig {
    host?: string;
    port?: number;
    maxConnections?: number;
    authToken?: string;
    rateLimit?: {
        maxRequestsPerMinute: number;
        maxBytesPerMinute: number;
    };
    tlsConfig?: {
        cert?: string;
        key?: string;
        ca?: string;
    };
}
export interface SyncRequest {
    type: 'episodes' | 'skills' | 'edges' | 'full';
    since?: number;
    filters?: Record<string, any>;
    batchSize?: number;
}
export interface SyncResponse {
    success: boolean;
    data?: any;
    error?: string;
    nextCursor?: number;
    hasMore?: boolean;
    count?: number;
}
interface ClientConnection {
    id: string;
    address: string;
    connectedAt: number;
    requestCount: number;
    bytesReceived: number;
    lastRequestAt: number;
}
export declare class QUICServer {
    private db;
    private config;
    private isRunning;
    private connections;
    private rateLimitState;
    private server;
    private cleanupInterval;
    constructor(db: Database, config?: QUICServerConfig);
    /**
     * Start the QUIC server
     */
    start(): Promise<void>;
    /**
     * Stop the QUIC server
     */
    stop(): Promise<void>;
    /**
     * Handle incoming client connection
     */
    private handleConnection;
    /**
     * Authenticate client request
     */
    private authenticate;
    /**
     * Check rate limits for client
     */
    private checkRateLimit;
    /**
     * Process sync request from client
     */
    processSyncRequest(clientId: string, request: SyncRequest, authToken: string): Promise<SyncResponse>;
    /**
     * Sync episodes data
     */
    private syncEpisodes;
    /**
     * Sync skills data
     */
    private syncSkills;
    /**
     * Sync edges (skill relationships)
     */
    private syncEdges;
    /**
     * Full sync of all data
     */
    private syncFull;
    /**
     * Start cleanup interval for stale connections
     */
    private startCleanupInterval;
    /**
     * Get server status
     */
    getStatus(): {
        isRunning: boolean;
        activeConnections: number;
        totalRequests: number;
        config: QUICServerConfig;
    };
    /**
     * Get connection info
     */
    getConnections(): ClientConnection[];
}
export {};
//# sourceMappingURL=QUICServer.d.ts.map