/**
 * OpenTelemetry Observability for AgentDB
 *
 * Provides comprehensive metrics, traces, and logging for production monitoring.
 * Zero-dependency mode when disabled, graceful degradation if collector unavailable.
 */
/**
 * Default telemetry configuration
 */
const DEFAULT_CONFIG = {
    enabled: process.env.NODE_ENV === 'production',
    serviceName: 'agentdb',
    serviceVersion: '2.0.0-alpha',
    otlpTraceEndpoint: process.env.OTEL_EXPORTER_OTLP_TRACES_ENDPOINT || 'http://localhost:4318/v1/traces',
    otlpMetricsEndpoint: process.env.OTEL_EXPORTER_OTLP_METRICS_ENDPOINT || 'http://localhost:4318/v1/metrics',
    prometheusEnabled: true,
    prometheusPort: 9464,
    consoleEnabled: process.env.NODE_ENV === 'development',
    samplingRate: 1.0,
    autoInstrumentation: true,
    resourceAttributes: {},
};
/**
 * Telemetry manager class
 */
export class TelemetryManager {
    static instance;
    sdk = null;
    tracer = null;
    meter = null;
    config;
    // Metrics
    queryLatencyHistogram;
    cacheHitCounter;
    cacheMissCounter;
    errorCounter;
    operationCounter;
    throughputCounter;
    cacheHitRateGauge;
    // Statistics for gauge calculation
    stats = {
        cacheHits: 0,
        cacheMisses: 0,
    };
    constructor(config = {}) {
        this.config = { ...DEFAULT_CONFIG, ...config };
    }
    /**
     * Get singleton instance
     */
    static getInstance(config) {
        if (!TelemetryManager.instance) {
            TelemetryManager.instance = new TelemetryManager(config);
        }
        return TelemetryManager.instance;
    }
    /**
     * Initialize telemetry
     */
    async initialize() {
        if (!this.config.enabled) {
            console.log('[AgentDB] Telemetry disabled');
            return;
        }
        try {
            // Dynamic import of OpenTelemetry packages
            const otelApi = await import('@opentelemetry/api');
            const { NodeSDK } = await import('@opentelemetry/sdk-node');
            const { Resource } = await import('@opentelemetry/resources');
            const { SemanticResourceAttributes } = await import('@opentelemetry/semantic-conventions');
            // Create resource with service information
            const resource = new Resource({
                [SemanticResourceAttributes.SERVICE_NAME]: this.config.serviceName,
                [SemanticResourceAttributes.SERVICE_VERSION]: this.config.serviceVersion,
                ...this.config.resourceAttributes,
            });
            // Basic SDK initialization without complex exporters for now
            this.sdk = new NodeSDK({
                resource,
                instrumentations: [],
            });
            await this.sdk.start();
            // Initialize tracer and meter
            this.tracer = otelApi.trace.getTracer(this.config.serviceName, this.config.serviceVersion);
            this.meter = otelApi.metrics.getMeter(this.config.serviceName, this.config.serviceVersion);
            // Initialize metrics
            this.initializeMetrics();
            console.log('[AgentDB] Telemetry initialized successfully');
        }
        catch (error) {
            console.error('[AgentDB] Failed to initialize telemetry:', error);
            console.log('[AgentDB] Continuing without telemetry (graceful degradation)');
            // Graceful degradation - continue without telemetry
            this.config.enabled = false;
        }
    }
    /**
     * Initialize metric instruments
     */
    initializeMetrics() {
        if (!this.meter)
            return;
        // Query latency histogram
        this.queryLatencyHistogram = this.meter.createHistogram('agentdb.query.latency', {
            description: 'Query execution latency in milliseconds',
            unit: 'ms',
        });
        // Cache hit counter
        this.cacheHitCounter = this.meter.createCounter('agentdb.cache.hits', {
            description: 'Number of cache hits',
        });
        // Cache miss counter
        this.cacheMissCounter = this.meter.createCounter('agentdb.cache.misses', {
            description: 'Number of cache misses',
        });
        // Error counter
        this.errorCounter = this.meter.createCounter('agentdb.errors', {
            description: 'Number of errors',
        });
        // Operation counter
        this.operationCounter = this.meter.createCounter('agentdb.operations', {
            description: 'Number of database operations',
        });
        // Throughput counter
        this.throughputCounter = this.meter.createCounter('agentdb.throughput', {
            description: 'Number of operations completed',
            unit: 'operations',
        });
        // Cache hit rate gauge
        this.cacheHitRateGauge = this.meter.createObservableGauge('agentdb.cache.hit_rate', {
            description: 'Cache hit rate percentage',
            unit: '%',
        });
        this.cacheHitRateGauge.addCallback((observableResult) => {
            const total = this.stats.cacheHits + this.stats.cacheMisses;
            const hitRate = total > 0 ? (this.stats.cacheHits / total) * 100 : 0;
            observableResult.observe(hitRate);
        });
    }
    /**
     * Get tracer instance
     */
    getTracer() {
        return this.tracer;
    }
    /**
     * Start a new span
     */
    startSpan(name, attributes) {
        if (!this.config.enabled || !this.tracer) {
            return null;
        }
        return this.tracer.startSpan(name, {
            attributes,
        });
    }
    /**
     * Record query latency
     */
    recordQueryLatency(latencyMs, operationType, tableName, success = true) {
        if (!this.config.enabled || !this.queryLatencyHistogram)
            return;
        this.queryLatencyHistogram.record(latencyMs, {
            operation_type: operationType,
            table_name: tableName || 'unknown',
            success: success.toString(),
        });
    }
    /**
     * Record cache hit
     */
    recordCacheHit(key) {
        if (!this.config.enabled || !this.cacheHitCounter)
            return;
        this.stats.cacheHits++;
        this.cacheHitCounter.add(1, { key_type: this.getKeyType(key) });
    }
    /**
     * Record cache miss
     */
    recordCacheMiss(key) {
        if (!this.config.enabled || !this.cacheMissCounter)
            return;
        this.stats.cacheMisses++;
        this.cacheMissCounter.add(1, { key_type: this.getKeyType(key) });
    }
    /**
     * Record error
     */
    recordError(errorType, operation, message) {
        if (!this.config.enabled || !this.errorCounter)
            return;
        this.errorCounter.add(1, {
            error_type: errorType,
            operation,
            message: message || 'unknown',
        });
    }
    /**
     * Record operation
     */
    recordOperation(operationType, tableName, resultSize) {
        if (!this.config.enabled || !this.operationCounter)
            return;
        this.operationCounter.add(1, {
            operation_type: operationType,
            table_name: tableName || 'unknown',
            result_size: resultSize?.toString() || '0',
        });
        if (this.throughputCounter) {
            this.throughputCounter.add(1, {
                operation_type: operationType,
            });
        }
    }
    /**
     * Get key type from cache key
     */
    getKeyType(key) {
        if (key.startsWith('query:'))
            return 'query';
        if (key.startsWith('skill:'))
            return 'skill';
        if (key.startsWith('episode:'))
            return 'episode';
        return 'other';
    }
    /**
     * Shutdown telemetry
     */
    async shutdown() {
        if (this.sdk) {
            await this.sdk.shutdown();
            console.log('[AgentDB] Telemetry shut down');
        }
    }
    /**
     * Check if telemetry is enabled
     */
    isEnabled() {
        return this.config.enabled;
    }
    /**
     * Reset statistics (for testing)
     */
    resetStats() {
        this.stats = {
            cacheHits: 0,
            cacheMisses: 0,
        };
    }
}
/**
 * Helper function to record metrics
 */
export function recordMetric(type, data) {
    const telemetry = TelemetryManager.getInstance();
    switch (type) {
        case 'query':
            if (data.latencyMs !== undefined && data.operationType) {
                telemetry.recordQueryLatency(data.latencyMs, data.operationType, data.tableName, data.success);
            }
            break;
        case 'cache_hit':
            if (data.key) {
                telemetry.recordCacheHit(data.key);
            }
            break;
        case 'cache_miss':
            if (data.key) {
                telemetry.recordCacheMiss(data.key);
            }
            break;
        case 'error':
            if (data.errorType && data.operation) {
                telemetry.recordError(data.errorType, data.operation, data.message);
            }
            break;
        case 'operation':
            if (data.operationType) {
                telemetry.recordOperation(data.operationType, data.tableName, data.resultSize);
            }
            break;
    }
}
/**
 * Decorator for automatic tracing
 */
export function traced(operationName, options = {}) {
    return function (target, propertyKey, descriptor) {
        const originalMethod = descriptor.value;
        const spanName = operationName || `${target.constructor.name}.${propertyKey}`;
        descriptor.value = async function (...args) {
            const telemetry = TelemetryManager.getInstance();
            if (!telemetry.isEnabled()) {
                return originalMethod.apply(this, args);
            }
            const startTime = Date.now();
            const span = telemetry.startSpan(spanName, options.attributes);
            try {
                const result = await originalMethod.apply(this, args);
                if (span) {
                    try {
                        span.setStatus({ code: 1 }); // OK status code
                        span.setAttribute('result_size', JSON.stringify(result).length);
                    }
                    catch {
                        // Ignore span errors
                    }
                }
                if (options.recordMetrics) {
                    const latency = Date.now() - startTime;
                    telemetry.recordQueryLatency(latency, spanName, undefined, true);
                    telemetry.recordOperation(spanName);
                }
                return result;
            }
            catch (error) {
                if (span) {
                    try {
                        span.setStatus({
                            code: 2, // ERROR status code
                            message: error instanceof Error ? error.message : 'Unknown error',
                        });
                        if (span.recordException) {
                            span.recordException(error);
                        }
                    }
                    catch {
                        // Ignore span errors
                    }
                }
                if (options.recordMetrics) {
                    const latency = Date.now() - startTime;
                    telemetry.recordQueryLatency(latency, spanName, undefined, false);
                    telemetry.recordError(error instanceof Error ? error.name : 'UnknownError', spanName, error instanceof Error ? error.message : 'Unknown error');
                }
                throw error;
            }
            finally {
                if (span && span.end) {
                    try {
                        span.end();
                    }
                    catch {
                        // Ignore span errors
                    }
                }
            }
        };
        return descriptor;
    };
}
/**
 * Manual span wrapper for non-decorator usage
 */
export async function withSpan(spanName, fn, attributes) {
    const telemetry = TelemetryManager.getInstance();
    if (!telemetry.isEnabled()) {
        return fn(null);
    }
    const span = telemetry.startSpan(spanName, attributes);
    try {
        const result = await fn(span);
        if (span && span.setStatus) {
            try {
                span.setStatus({ code: 1 }); // OK status code
            }
            catch {
                // Ignore
            }
        }
        return result;
    }
    catch (error) {
        if (span) {
            try {
                if (span.setStatus) {
                    span.setStatus({
                        code: 2, // ERROR status code
                        message: error instanceof Error ? error.message : 'Unknown error',
                    });
                }
                if (span.recordException) {
                    span.recordException(error);
                }
            }
            catch {
                // Ignore
            }
        }
        throw error;
    }
    finally {
        if (span && span.end) {
            try {
                span.end();
            }
            catch {
                // Ignore
            }
        }
    }
}
// Initialize telemetry on module load
const telemetry = TelemetryManager.getInstance();
telemetry.initialize().catch(console.error);
// Graceful shutdown on process termination
process.on('SIGTERM', async () => {
    await telemetry.shutdown();
});
process.on('SIGINT', async () => {
    await telemetry.shutdown();
});
export default TelemetryManager;
//# sourceMappingURL=telemetry.js.map