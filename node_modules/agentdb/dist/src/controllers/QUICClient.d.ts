/**
 * QUICClient - QUIC Protocol Client for AgentDB Synchronization
 *
 * Implements a QUIC client for initiating synchronization requests to remote
 * AgentDB instances. Supports connection pooling, retry logic, and reliable sync.
 *
 * Features:
 * - Connect to remote QUIC servers
 * - Send sync requests (episodes, skills, edges)
 * - Handle responses and errors
 * - Automatic retry with exponential backoff
 * - Connection pooling for efficiency
 * - Comprehensive error handling
 */
export interface QUICClientConfig {
    serverHost: string;
    serverPort: number;
    authToken?: string;
    maxRetries?: number;
    retryDelayMs?: number;
    timeoutMs?: number;
    poolSize?: number;
    tlsConfig?: {
        cert?: string;
        key?: string;
        ca?: string;
        rejectUnauthorized?: boolean;
    };
}
export interface SyncOptions {
    type: 'episodes' | 'skills' | 'edges' | 'full';
    since?: number;
    filters?: Record<string, any>;
    batchSize?: number;
    onProgress?: (progress: SyncProgress) => void;
}
export interface SyncProgress {
    phase: 'connecting' | 'syncing' | 'processing' | 'completed' | 'error';
    itemsSynced?: number;
    totalItems?: number;
    bytesTransferred?: number;
    error?: string;
}
export interface SyncResult {
    success: boolean;
    data?: any;
    itemsReceived: number;
    bytesTransferred: number;
    durationMs: number;
    error?: string;
}
export interface PushOptions {
    type: 'episodes' | 'skills' | 'edges';
    data: any[];
    batchSize?: number;
    onProgress?: (progress: PushProgress) => void;
}
export interface PushProgress {
    phase: 'connecting' | 'pushing' | 'processing' | 'completed' | 'error';
    itemsPushed?: number;
    totalItems?: number;
    bytesTransferred?: number;
    currentBatch?: number;
    totalBatches?: number;
    error?: string;
}
export interface PushResult {
    success: boolean;
    itemsPushed: number;
    bytesTransferred: number;
    durationMs: number;
    error?: string;
    failedItems?: any[];
}
export declare class QUICClient {
    private config;
    private connectionPool;
    private isConnected;
    private retryCount;
    constructor(config: QUICClientConfig);
    /**
     * Connect to remote QUIC server
     */
    connect(): Promise<void>;
    /**
     * Disconnect from server
     */
    disconnect(): Promise<void>;
    /**
     * Send sync request to server
     */
    sync(options: SyncOptions): Promise<SyncResult>;
    /**
     * Send request with automatic retry
     */
    private sendWithRetry;
    /**
     * Send request to server
     */
    private sendRequest;
    /**
     * Acquire connection from pool
     */
    private acquireConnection;
    /**
     * Release connection back to pool
     */
    private releaseConnection;
    /**
     * Get client status
     */
    getStatus(): {
        isConnected: boolean;
        poolSize: number;
        activeConnections: number;
        totalRequests: number;
        config: QUICClientConfig;
    };
    /**
     * Test connection to server
     */
    ping(): Promise<{
        success: boolean;
        latencyMs: number;
        error?: string;
    }>;
    /**
     * Push data to remote server
     */
    push(options: PushOptions): Promise<PushResult>;
    /**
     * Push multiple data types in a single operation
     */
    pushAll(data: {
        episodes?: any[];
        skills?: any[];
        edges?: any[];
    }, options?: {
        batchSize?: number;
        onProgress?: (type: string, progress: PushProgress) => void;
    }): Promise<{
        success: boolean;
        results: Record<string, PushResult>;
        totalItemsPushed: number;
        totalBytesTransferred: number;
        totalDurationMs: number;
        errors: string[];
    }>;
    /**
     * Sleep helper
     */
    private sleep;
}
//# sourceMappingURL=QUICClient.d.ts.map