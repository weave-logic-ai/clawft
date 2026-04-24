/**
 * AgentDB v2 Resource Limit Enforcement
 *
 * Prevents denial of service attacks by enforcing:
 * - Memory usage limits
 * - Query timeouts
 * - Rate limiting
 * - Resource caps
 */
/**
 * Resource usage tracker
 */
export declare class ResourceTracker {
    private memoryUsageMB;
    private queryCount;
    private lastQueryTime;
    private queryTimes;
    private readonly startTime;
    /**
     * Update memory usage estimate
     */
    updateMemoryUsage(additionalMB: number): void;
    /**
     * Estimate memory for vectors
     */
    estimateVectorMemory(numVectors: number, dimension: number): number;
    /**
     * Record query execution
     */
    recordQuery(durationMs: number): void;
    /**
     * Get resource usage statistics
     */
    getStats(): ResourceStats;
    /**
     * Reset tracker
     */
    reset(): void;
}
/**
 * Resource statistics interface
 */
export interface ResourceStats {
    memoryUsageMB: number;
    memoryLimitMB: number;
    memoryUtilization: number;
    queryCount: number;
    avgQueryTimeMs: number;
    uptimeSeconds: number;
    queriesPerSecond: number;
}
/**
 * Query timeout wrapper
 */
export declare function withTimeout<T>(promise: Promise<T>, timeoutMs?: number, operation?: string): Promise<T>;
/**
 * Rate limiter using token bucket algorithm
 */
export declare class RateLimiter {
    private maxTokens;
    private refillRate;
    private tokens;
    private lastRefill;
    constructor(maxTokens: number, refillRate: number);
    /**
     * Attempt to consume tokens
     * @returns true if allowed, false if rate limited
     */
    tryConsume(tokens?: number): boolean;
    /**
     * Consume tokens or throw error
     */
    consume(tokens?: number, operation?: string): void;
    /**
     * Refill tokens based on time elapsed
     */
    private refill;
    /**
     * Get current token count
     */
    getTokens(): number;
    /**
     * Reset limiter
     */
    reset(): void;
}
/**
 * Security error class
 */
export declare class SecurityError extends Error {
    readonly code: string;
    constructor(message: string, code: string);
    /**
     * Get safe error message for external consumption
     */
    getSafeMessage(): string;
}
/**
 * Enforce resource limits on batch operations
 */
export declare function enforceBatchLimits(batchSize: number, dimension: number, tracker: ResourceTracker): void;
/**
 * Circuit breaker for fault tolerance
 */
export declare class CircuitBreaker {
    private maxFailures;
    private resetTimeoutMs;
    private failures;
    private lastFailureTime;
    private state;
    constructor(maxFailures?: number, resetTimeoutMs?: number);
    /**
     * Execute operation with circuit breaker protection
     */
    execute<T>(operation: () => Promise<T>, operationName?: string): Promise<T>;
    /**
     * Record a failure
     */
    private recordFailure;
    /**
     * Get circuit breaker status
     */
    getStatus(): {
        state: string;
        failures: number;
        lastFailure?: number;
    };
    /**
     * Manually reset circuit breaker
     */
    reset(): void;
}
/**
 * Global resource tracker instance
 */
export declare const globalResourceTracker: ResourceTracker;
/**
 * Default rate limiters
 */
export declare const rateLimiters: {
    insert: RateLimiter;
    search: RateLimiter;
    delete: RateLimiter;
    batch: RateLimiter;
};
/**
 * Monitor and log resource usage
 */
export declare function logResourceUsage(): void;
//# sourceMappingURL=limits.d.ts.map