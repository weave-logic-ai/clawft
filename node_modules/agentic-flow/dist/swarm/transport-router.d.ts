import { QuicCoordinator, SwarmMessage, SwarmAgent } from './quic-coordinator.js';
export type TransportProtocol = 'quic' | 'http2' | 'auto';
export interface TransportConfig {
    protocol: TransportProtocol;
    enableFallback: boolean;
    quicConfig?: {
        host: string;
        port: number;
        maxConnections: number;
        certPath?: string;
        keyPath?: string;
    };
    http2Config?: {
        host: string;
        port: number;
        maxConnections: number;
        secure: boolean;
    };
}
export interface TransportStats {
    protocol: 'quic' | 'http2';
    messagesSent: number;
    messagesReceived: number;
    bytesTransferred: number;
    averageLatency: number;
    errorRate: number;
}
export interface RouteResult {
    success: boolean;
    protocol: 'quic' | 'http2';
    latency: number;
    error?: string;
}
/**
 * TransportRouter - Intelligent transport layer with automatic protocol selection
 *
 * Features:
 * - Automatic QUIC/HTTP2 protocol selection
 * - Transparent fallback on failure
 * - Connection pooling for both protocols
 * - Per-protocol statistics tracking
 * - Health checking and availability detection
 */
export declare class TransportRouter {
    private config;
    private quicClient?;
    private quicPool?;
    private quicCoordinator?;
    private http2Sessions;
    private currentProtocol;
    private stats;
    private healthCheckTimer?;
    private quicAvailable;
    constructor(config: TransportConfig);
    /**
     * Initialize transport router
     */
    initialize(): Promise<void>;
    /**
     * Initialize QUIC transport
     */
    private initializeQuic;
    /**
     * Initialize HTTP/2 transport
     */
    private initializeHttp2;
    /**
     * Initialize QUIC coordinator for swarm
     */
    initializeSwarm(swarmId: string, topology: 'mesh' | 'hierarchical' | 'ring' | 'star', maxAgents?: number): Promise<QuicCoordinator>;
    /**
     * Route message through appropriate transport
     */
    route(message: SwarmMessage, target: SwarmAgent): Promise<RouteResult>;
    /**
     * Send message via QUIC
     */
    private sendViaQuic;
    /**
     * Send message via HTTP/2
     */
    private sendViaHttp2;
    /**
     * Get current transport protocol
     */
    getCurrentProtocol(): 'quic' | 'http2';
    /**
     * Check if QUIC is available
     */
    isQuicAvailable(): boolean;
    /**
     * Get transport statistics
     */
    getStats(protocol?: 'quic' | 'http2'): TransportStats | Map<'quic' | 'http2', TransportStats>;
    /**
     * Get QUIC coordinator (if initialized)
     */
    getCoordinator(): QuicCoordinator | undefined;
    /**
     * Shutdown transport router
     */
    shutdown(): Promise<void>;
    /**
     * Start health checks for protocol availability
     */
    private startHealthChecks;
    /**
     * Check QUIC health
     */
    private checkQuicHealth;
    /**
     * Update transport statistics
     */
    private updateStats;
    /**
     * Serialize message to bytes
     */
    private serializeMessage;
    /**
     * Estimate message size in bytes
     */
    private estimateMessageSize;
}
//# sourceMappingURL=transport-router.d.ts.map