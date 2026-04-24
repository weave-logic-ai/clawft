/**
 * Performance Metrics and Monitoring for Attention Mechanisms
 * Real-time tracking of latency, throughput, and memory usage
 */
export interface AttentionMetrics {
    mechanism: string;
    operationCount: number;
    totalLatencyMs: number;
    avgLatencyUs: number;
    minLatencyUs: number;
    maxLatencyUs: number;
    p50LatencyUs: number;
    p95LatencyUs: number;
    p99LatencyUs: number;
    throughputOpsPerSec: number;
    memoryUsageBytes: number;
    peakMemoryBytes: number;
    allocationCount: number;
}
export interface OperationMetrics {
    startTime: number;
    endTime: number;
    latencyUs: number;
    memoryBefore: number;
    memoryAfter: number;
    memoryDelta: number;
}
export declare class AttentionMetricsCollector {
    private metrics;
    private startMemory;
    /**
     * Start measuring an operation
     */
    startOperation(mechanism: string): void;
    /**
     * End measuring an operation and record metrics
     */
    endOperation(mechanism: string, startTime: number): void;
    /**
     * Get aggregated metrics for a mechanism
     */
    getMetrics(mechanism: string): AttentionMetrics | null;
    /**
     * Get all collected metrics
     */
    getAllMetrics(): Map<string, AttentionMetrics>;
    /**
     * Reset all metrics
     */
    reset(): void;
    /**
     * Export metrics as JSON
     */
    exportJSON(): string;
    /**
     * Export metrics in markdown format
     */
    exportMarkdown(): string;
}
/**
 * Global metrics collector instance
 */
export declare const metricsCollector: AttentionMetricsCollector;
/**
 * Decorator for measuring function performance
 */
export declare function measurePerformance(mechanism: string): (target: any, propertyKey: string, descriptor: PropertyDescriptor) => PropertyDescriptor;
/**
 * Helper to measure a synchronous operation
 */
export declare function measureSync<T>(mechanism: string, operation: () => T): T;
/**
 * Helper to measure an async operation
 */
export declare function measureAsync<T>(mechanism: string, operation: () => Promise<T>): Promise<T>;
//# sourceMappingURL=attention-metrics.d.ts.map