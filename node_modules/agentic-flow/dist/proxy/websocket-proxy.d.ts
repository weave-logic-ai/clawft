/**
 * WebSocket Proxy for LLM Streaming
 *
 * Features:
 * - Bidirectional: Full-duplex communication
 * - Mobile-friendly: Better for unstable connections
 * - Lower overhead: No HTTP headers per message
 * - Reconnection: Automatic retry on disconnect
 * - Universal support: Works everywhere (browsers, mobile, desktop)
 *
 * Use Case: Fallback for unreliable connections (mobile, poor WiFi)
 */
export interface WebSocketProxyConfig {
    port: number;
    geminiApiKey?: string;
    geminiBaseUrl?: string;
    pingInterval?: number;
    pingTimeout?: number;
    maxConnections?: number;
    connectionTimeout?: number;
}
export declare class WebSocketProxy {
    private wss;
    private server;
    private config;
    private clients;
    private pingInterval?;
    private activeConnections;
    constructor(config: WebSocketProxyConfig);
    private setupHandlers;
    private setupHeartbeat;
    private handleStreamingRequest;
    private handleNonStreamingRequest;
    private convertAnthropicToGemini;
    private convertGeminiToAnthropic;
    start(): Promise<void>;
    stop(): Promise<void>;
}
//# sourceMappingURL=websocket-proxy.d.ts.map