/**
 * OpenTelemetry Observability for AgentDB
 *
 * Provides comprehensive metrics, traces, and logging for production monitoring.
 * Zero-dependency mode when disabled, graceful degradation if collector unavailable.
 */
type Span = any;
type Tracer = any;
/**
 * Telemetry configuration interface
 */
export interface TelemetryConfig {
    /** Enable/disable telemetry (default: enabled in production) */
    enabled?: boolean;
    /** Service name */
    serviceName?: string;
    /** Service version */
    serviceVersion?: string;
    /** OTLP endpoint for traces */
    otlpTraceEndpoint?: string;
    /** OTLP endpoint for metrics */
    otlpMetricsEndpoint?: string;
    /** Enable Prometheus metrics exporter */
    prometheusEnabled?: boolean;
    /** Prometheus port */
    prometheusPort?: number;
    /** Enable console exporters for development */
    consoleEnabled?: boolean;
    /** Sampling rate (0.0 to 1.0, default: 1.0 for alpha) */
    samplingRate?: number;
    /** Enable automatic instrumentation */
    autoInstrumentation?: boolean;
    /** Custom resource attributes */
    resourceAttributes?: Record<string, string>;
}
/**
 * Telemetry manager class
 */
export declare class TelemetryManager {
    private static instance;
    private sdk;
    private tracer;
    private meter;
    private config;
    private queryLatencyHistogram?;
    private cacheHitCounter?;
    private cacheMissCounter?;
    private errorCounter?;
    private operationCounter?;
    private throughputCounter?;
    private cacheHitRateGauge?;
    private stats;
    private constructor();
    /**
     * Get singleton instance
     */
    static getInstance(config?: TelemetryConfig): TelemetryManager;
    /**
     * Initialize telemetry
     */
    initialize(): Promise<void>;
    /**
     * Initialize metric instruments
     */
    private initializeMetrics;
    /**
     * Get tracer instance
     */
    getTracer(): Tracer | null;
    /**
     * Start a new span
     */
    startSpan(name: string, attributes?: Record<string, any>): Span | null;
    /**
     * Record query latency
     */
    recordQueryLatency(latencyMs: number, operationType: string, tableName?: string, success?: boolean): void;
    /**
     * Record cache hit
     */
    recordCacheHit(key: string): void;
    /**
     * Record cache miss
     */
    recordCacheMiss(key: string): void;
    /**
     * Record error
     */
    recordError(errorType: string, operation: string, message?: string): void;
    /**
     * Record operation
     */
    recordOperation(operationType: string, tableName?: string, resultSize?: number): void;
    /**
     * Get key type from cache key
     */
    private getKeyType;
    /**
     * Shutdown telemetry
     */
    shutdown(): Promise<void>;
    /**
     * Check if telemetry is enabled
     */
    isEnabled(): boolean;
    /**
     * Reset statistics (for testing)
     */
    resetStats(): void;
}
/**
 * Helper function to record metrics
 */
export declare function recordMetric(type: 'query' | 'cache_hit' | 'cache_miss' | 'error' | 'operation', data: {
    latencyMs?: number;
    operationType?: string;
    tableName?: string;
    success?: boolean;
    key?: string;
    errorType?: string;
    operation?: string;
    message?: string;
    resultSize?: number;
}): void;
/**
 * Decorator for automatic tracing
 */
export declare function traced(operationName?: string, options?: {
    attributes?: Record<string, any>;
    recordMetrics?: boolean;
}): (target: any, propertyKey: string, descriptor: PropertyDescriptor) => PropertyDescriptor;
/**
 * Manual span wrapper for non-decorator usage
 */
export declare function withSpan<T>(spanName: string, fn: (span: Span | null) => Promise<T>, attributes?: Record<string, any>): Promise<T>;
export default TelemetryManager;
//# sourceMappingURL=telemetry.d.ts.map