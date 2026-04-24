import { Response } from 'express';
import { QuicConfig } from '../transport/quic.js';
import { AnthropicToOpenRouterProxy } from './anthropic-to-openrouter.js';
export interface QuicProxyConfig {
    openrouterApiKey: string;
    openrouterBaseUrl?: string;
    defaultModel?: string;
    transport?: 'quic' | 'http2' | 'auto';
    enableQuic?: boolean;
    quic?: QuicConfig;
    fallbackToHttp2?: boolean;
    fallbackTimeout?: number;
}
export declare class QuicEnabledProxy extends AnthropicToOpenRouterProxy {
    private quicClient?;
    private quicPool?;
    private transport;
    private quicEnabled;
    private fallbackToHttp2;
    constructor(config: QuicProxyConfig);
    /**
     * Check if QUIC is enabled via environment variable
     */
    private checkQuicFeatureFlag;
    /**
     * Initialize QUIC client and connection pool
     */
    private initializeQuic;
    /**
     * Select transport protocol based on configuration and availability
     */
    private selectTransport;
    /**
     * Send request using selected transport
     */
    protected sendRequest(url: string, options: RequestInit): Promise<Response>;
    /**
     * Send request over QUIC
     */
    private sendQuicRequest;
    /**
     * Send request over HTTP/2 (standard fetch)
     */
    private sendHttp2Request;
    /**
     * Get transport statistics
     */
    getTransportStats(): any;
    /**
     * Shutdown and cleanup
     */
    shutdown(): Promise<void>;
}
/**
 * Create QUIC-enabled proxy with configuration
 */
export declare function createQuicProxy(config: QuicProxyConfig): QuicEnabledProxy;
//# sourceMappingURL=quic-proxy.d.ts.map