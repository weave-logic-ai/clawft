export interface QuicConfig {
    host?: string;
    port?: number;
    certPath?: string;
    keyPath?: string;
    serverHost?: string;
    serverPort?: number;
    verifyPeer?: boolean;
    maxConnections?: number;
    connectionTimeout?: number;
    idleTimeout?: number;
    maxConcurrentStreams?: number;
    streamTimeout?: number;
    initialCongestionWindow?: number;
    maxDatagramSize?: number;
    enableEarlyData?: boolean;
}
export interface QuicConnection {
    id: string;
    remoteAddr: string;
    streamCount: number;
    createdAt: Date;
    lastActivity: Date;
}
export interface QuicStream {
    id: number;
    connectionId: string;
    send(data: Uint8Array): Promise<void>;
    receive(): Promise<Uint8Array>;
    close(): Promise<void>;
}
export interface QuicStats {
    totalConnections: number;
    activeConnections: number;
    totalStreams: number;
    activeStreams: number;
    bytesReceived: number;
    bytesSent: number;
    packetsLost: number;
    rttMs: number;
}
/**
 * QUIC Client - Manages outbound QUIC connections and stream multiplexing
 */
export declare class QuicClient {
    private config;
    private connections;
    private wasmModule;
    private initialized;
    constructor(config?: QuicConfig);
    /**
     * Initialize QUIC client with WASM module
     */
    initialize(): Promise<void>;
    /**
     * Connect to QUIC server
     */
    connect(host?: string, port?: number): Promise<QuicConnection>;
    /**
     * Create bidirectional stream on connection
     */
    createStream(connectionId: string): Promise<QuicStream>;
    /**
     * Send HTTP/3 request over QUIC
     */
    sendRequest(connectionId: string, method: string, path: string, headers: Record<string, string>, body?: Uint8Array): Promise<{
        status: number;
        headers: Record<string, string>;
        body: Uint8Array;
    }>;
    /**
     * Close connection
     */
    closeConnection(connectionId: string): Promise<void>;
    /**
     * Close all connections and cleanup
     */
    shutdown(): Promise<void>;
    /**
     * Get connection statistics
     */
    getStats(): QuicStats;
    /**
     * Load WASM module (placeholder)
     */
    private loadWasmModule;
    /**
     * Encode HTTP/3 request (placeholder)
     */
    private encodeHttp3Request;
    /**
     * Decode HTTP/3 response (placeholder)
     */
    private decodeHttp3Response;
}
/**
 * QUIC Server - Listens for inbound QUIC connections
 */
export declare class QuicServer {
    private config;
    private connections;
    private wasmModule;
    private initialized;
    private listening;
    constructor(config?: QuicConfig);
    /**
     * Initialize QUIC server
     */
    initialize(): Promise<void>;
    /**
     * Start listening for connections
     */
    listen(): Promise<void>;
    /**
     * Stop server and close all connections
     */
    stop(): Promise<void>;
    /**
     * Close connection
     */
    closeConnection(connectionId: string): Promise<void>;
    /**
     * Get server statistics
     */
    getStats(): QuicStats;
    /**
     * Load WASM module (placeholder)
     */
    private loadWasmModule;
}
/**
 * Connection pool manager for QUIC connections
 */
export declare class QuicConnectionPool {
    private client;
    private connections;
    private maxPoolSize;
    constructor(client: QuicClient, maxPoolSize?: number);
    /**
     * Get or create connection from pool
     */
    getConnection(host: string, port: number): Promise<QuicConnection>;
    /**
     * Remove oldest idle connection
     */
    private removeOldestConnection;
    /**
     * Clear all connections in pool
     */
    clear(): Promise<void>;
}
/**
 * QuicTransport - High-level QUIC transport interface
 * Simplified API for common use cases (backwards compatible)
 */
export interface QuicTransportConfig {
    host?: string;
    port?: number;
    maxConcurrentStreams?: number;
    certPath?: string;
    keyPath?: string;
}
export declare class QuicTransport {
    private client;
    private config;
    constructor(config?: QuicTransportConfig);
    /**
     * Connect to QUIC server
     */
    connect(): Promise<void>;
    /**
     * Send data over QUIC
     */
    send(data: any): Promise<void>;
    /**
     * Close connection
     */
    close(): Promise<void>;
    /**
     * Get connection statistics
     */
    getStats(): QuicStats;
}
//# sourceMappingURL=quic.d.ts.map