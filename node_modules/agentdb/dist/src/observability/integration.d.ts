/**
 * Telemetry integration helpers
 *
 * Provides integration utilities for adding telemetry to AgentDB components.
 */
/**
 * Wrap a database operation with telemetry
 */
export declare function withTelemetry<T>(operationType: string, tableName: string, operation: () => Promise<T>): Promise<T>;
/**
 * Wrap a batch operation with telemetry
 */
export declare function withBatchTelemetry<T>(operationType: string, tableName: string, batchSize: number, operation: () => Promise<T>): Promise<T>;
/**
 * Record cache access with telemetry
 */
export declare function recordCacheAccess(key: string, hit: boolean): void;
/**
 * Create span attributes from operation context
 */
export declare function createSpanAttributes(context: {
    operation?: string;
    table?: string;
    filters?: Record<string, any>;
    limit?: number;
    offset?: number;
}): Record<string, any>;
/**
 * Enhanced error recording with context
 */
export declare function recordErrorWithContext(error: Error, operation: string, context?: Record<string, any>): void;
//# sourceMappingURL=integration.d.ts.map