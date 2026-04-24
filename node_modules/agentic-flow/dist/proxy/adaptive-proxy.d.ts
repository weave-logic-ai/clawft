/**
 * Adaptive Multi-Protocol Proxy
 *
 * Automatically selects optimal protocol based on:
 * - Client capabilities
 * - Network conditions
 * - Configuration priorities
 *
 * Fallback chain: HTTP/3 → HTTP/2 → HTTP/1.1 → WebSocket
 */
export interface AdaptiveProxyConfig {
    enableHTTP2?: boolean;
    enableHTTP3?: boolean;
    enableWebSocket?: boolean;
    enableHTTP1?: boolean;
    http1Port?: number;
    http2Port?: number;
    http3Port?: number;
    wsPort?: number;
    cert?: string;
    key?: string;
    geminiApiKey?: string;
    geminiBaseUrl?: string;
}
interface ProxyServer {
    protocol: string;
    port: number;
    url: string;
    proxy: any;
}
export declare class AdaptiveProxy {
    private config;
    private servers;
    private isRunning;
    constructor(config: AdaptiveProxyConfig);
    private getEnabledProtocols;
    start(): Promise<ProxyServer[]>;
    stop(): Promise<void>;
    getServers(): ProxyServer[];
    getStatus(): {
        isRunning: boolean;
        servers: Array<{
            protocol: string;
            port: number;
            url: string;
        }>;
        enabledProtocols: string[];
    };
}
export {};
//# sourceMappingURL=adaptive-proxy.d.ts.map