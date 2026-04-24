import { QuicConfig } from '../transport/quic.js';
export interface QuicConfigSchema extends QuicConfig {
    enabled: boolean;
    autoDetect: boolean;
    fallbackToHttp2: boolean;
    healthCheck: {
        enabled: boolean;
        interval: number;
        timeout: number;
    };
    monitoring: {
        enabled: boolean;
        logInterval: number;
    };
}
/**
 * Default QUIC configuration
 */
export declare const DEFAULT_QUIC_CONFIG: QuicConfigSchema;
/**
 * Load QUIC configuration from environment and config file
 */
export declare function loadQuicConfig(overrides?: Partial<QuicConfigSchema>): QuicConfigSchema;
/**
 * Validate QUIC configuration
 */
export declare function validateQuicConfig(config: QuicConfigSchema): void;
/**
 * Get QUIC configuration for router integration
 */
export declare function getQuicRouterConfig(config: QuicConfigSchema): {
    transport: {
        quic: {
            enabled: boolean;
            port: number;
            host: string;
            maxConnections: number;
            certPath: string;
            keyPath: string;
        };
        fallback: {
            enabled: boolean;
            protocol: string;
        };
    };
};
/**
 * Check if QUIC is available and properly configured
 */
export declare function checkQuicAvailability(): Promise<{
    available: boolean;
    reason?: string;
}>;
/**
 * Simplified config getter (alias for loadQuicConfig)
 */
export declare function getQuicConfig(overrides?: Partial<QuicConfigSchema>): QuicConfigSchema;
//# sourceMappingURL=quic.d.ts.map