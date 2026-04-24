/**
 * Connection Pool for HTTP/2 and HTTP/3 Proxies
 * Provides connection reuse to reduce latency by 20-30%
 */
import http2 from 'http2';
export interface PoolConfig {
    maxSize: number;
    maxIdleTime: number;
    acquireTimeout: number;
}
export declare class ConnectionPool {
    private pools;
    private config;
    private cleanupInterval;
    constructor(config?: Partial<PoolConfig>);
    acquire(host: string): Promise<http2.ClientHttp2Session>;
    release(session: http2.ClientHttp2Session, host: string): Promise<void>;
    private createConnection;
    private waitForConnection;
    private isExpired;
    private removeConnection;
    private cleanup;
    destroy(): void;
    getStats(): Record<string, {
        total: number;
        busy: number;
        idle: number;
    }>;
}
//# sourceMappingURL=connection-pool.d.ts.map