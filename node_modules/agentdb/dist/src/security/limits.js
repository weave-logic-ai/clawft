/**
 * AgentDB v2 Resource Limit Enforcement
 *
 * Prevents denial of service attacks by enforcing:
 * - Memory usage limits
 * - Query timeouts
 * - Rate limiting
 * - Resource caps
 */
import { SECURITY_LIMITS } from './validation.js';
/**
 * Resource usage tracker
 */
export class ResourceTracker {
    memoryUsageMB = 0;
    queryCount = 0;
    lastQueryTime = Date.now();
    queryTimes = [];
    startTime = Date.now();
    /**
     * Update memory usage estimate
     */
    updateMemoryUsage(additionalMB) {
        this.memoryUsageMB += additionalMB;
        if (this.memoryUsageMB > SECURITY_LIMITS.MAX_MEMORY_MB) {
            throw new SecurityError(`Memory limit exceeded: ${this.memoryUsageMB.toFixed(2)}MB > ${SECURITY_LIMITS.MAX_MEMORY_MB}MB`, 'MEMORY_LIMIT_EXCEEDED');
        }
    }
    /**
     * Estimate memory for vectors
     */
    estimateVectorMemory(numVectors, dimension) {
        // Float32Array: 4 bytes per value
        // Plus overhead for object structure and metadata (~25%)
        const bytesPerVector = dimension * 4 * 1.25;
        const totalBytes = numVectors * bytesPerVector;
        return totalBytes / (1024 * 1024); // Convert to MB
    }
    /**
     * Record query execution
     */
    recordQuery(durationMs) {
        this.queryCount++;
        this.lastQueryTime = Date.now();
        this.queryTimes.push(durationMs);
        // Keep only last 100 query times for stats
        if (this.queryTimes.length > 100) {
            this.queryTimes.shift();
        }
    }
    /**
     * Get resource usage statistics
     */
    getStats() {
        const avgQueryTime = this.queryTimes.length > 0
            ? this.queryTimes.reduce((a, b) => a + b, 0) / this.queryTimes.length
            : 0;
        const uptimeSeconds = (Date.now() - this.startTime) / 1000;
        return {
            memoryUsageMB: this.memoryUsageMB,
            memoryLimitMB: SECURITY_LIMITS.MAX_MEMORY_MB,
            memoryUtilization: (this.memoryUsageMB / SECURITY_LIMITS.MAX_MEMORY_MB) * 100,
            queryCount: this.queryCount,
            avgQueryTimeMs: avgQueryTime,
            uptimeSeconds,
            queriesPerSecond: this.queryCount / uptimeSeconds,
        };
    }
    /**
     * Reset tracker
     */
    reset() {
        this.memoryUsageMB = 0;
        this.queryCount = 0;
        this.queryTimes = [];
    }
}
/**
 * Query timeout wrapper
 */
export async function withTimeout(promise, timeoutMs = SECURITY_LIMITS.QUERY_TIMEOUT_MS, operation = 'operation') {
    const timeout = new Promise((_, reject) => {
        setTimeout(() => {
            reject(new SecurityError(`${operation} timeout after ${timeoutMs}ms`, 'QUERY_TIMEOUT'));
        }, timeoutMs);
    });
    return Promise.race([promise, timeout]);
}
/**
 * Rate limiter using token bucket algorithm
 */
export class RateLimiter {
    maxTokens;
    refillRate;
    tokens;
    lastRefill;
    constructor(maxTokens, refillRate) {
        this.maxTokens = maxTokens;
        this.refillRate = refillRate;
        this.tokens = maxTokens;
        this.lastRefill = Date.now();
    }
    /**
     * Attempt to consume tokens
     * @returns true if allowed, false if rate limited
     */
    tryConsume(tokens = 1) {
        this.refill();
        if (this.tokens >= tokens) {
            this.tokens -= tokens;
            return true;
        }
        return false;
    }
    /**
     * Consume tokens or throw error
     */
    consume(tokens = 1, operation = 'operation') {
        if (!this.tryConsume(tokens)) {
            throw new SecurityError(`Rate limit exceeded for ${operation}. Try again later.`, 'RATE_LIMIT_EXCEEDED');
        }
    }
    /**
     * Refill tokens based on time elapsed
     */
    refill() {
        const now = Date.now();
        const elapsed = (now - this.lastRefill) / 1000; // seconds
        const tokensToAdd = elapsed * this.refillRate;
        this.tokens = Math.min(this.maxTokens, this.tokens + tokensToAdd);
        this.lastRefill = now;
    }
    /**
     * Get current token count
     */
    getTokens() {
        this.refill();
        return this.tokens;
    }
    /**
     * Reset limiter
     */
    reset() {
        this.tokens = this.maxTokens;
        this.lastRefill = Date.now();
    }
}
/**
 * Security error class
 */
export class SecurityError extends Error {
    code;
    constructor(message, code) {
        super(message);
        this.code = code;
        this.name = 'SecurityError';
    }
    /**
     * Get safe error message for external consumption
     */
    getSafeMessage() {
        // Don't expose internal details in production
        return 'A security constraint was violated. Please check your request.';
    }
}
/**
 * Enforce resource limits on batch operations
 */
export function enforceBatchLimits(batchSize, dimension, tracker) {
    // Check batch size
    if (batchSize > SECURITY_LIMITS.MAX_BATCH_SIZE) {
        throw new SecurityError(`Batch size ${batchSize} exceeds limit ${SECURITY_LIMITS.MAX_BATCH_SIZE}`, 'BATCH_TOO_LARGE');
    }
    // Estimate and check memory
    const estimatedMemoryMB = tracker.estimateVectorMemory(batchSize, dimension);
    if (estimatedMemoryMB > SECURITY_LIMITS.MAX_MEMORY_MB * 0.5) {
        throw new SecurityError(`Batch operation would use ${estimatedMemoryMB.toFixed(2)}MB (>50% of limit)`, 'BATCH_MEMORY_EXCESSIVE');
    }
    tracker.updateMemoryUsage(estimatedMemoryMB);
}
/**
 * Circuit breaker for fault tolerance
 */
export class CircuitBreaker {
    maxFailures;
    resetTimeoutMs;
    failures = 0;
    lastFailureTime = 0;
    state = 'closed';
    constructor(maxFailures = 5, resetTimeoutMs = 60000) {
        this.maxFailures = maxFailures;
        this.resetTimeoutMs = resetTimeoutMs;
    }
    /**
     * Execute operation with circuit breaker protection
     */
    async execute(operation, operationName = 'operation') {
        if (this.state === 'open') {
            const timeSinceFailure = Date.now() - this.lastFailureTime;
            if (timeSinceFailure > this.resetTimeoutMs) {
                this.state = 'half-open';
            }
            else {
                throw new SecurityError(`Circuit breaker open for ${operationName}. Service temporarily unavailable.`, 'CIRCUIT_BREAKER_OPEN');
            }
        }
        try {
            const result = await operation();
            // Success - reset on half-open or keep closed
            if (this.state === 'half-open') {
                this.state = 'closed';
                this.failures = 0;
            }
            return result;
        }
        catch (error) {
            this.recordFailure();
            throw error;
        }
    }
    /**
     * Record a failure
     */
    recordFailure() {
        this.failures++;
        this.lastFailureTime = Date.now();
        if (this.failures >= this.maxFailures) {
            this.state = 'open';
            console.error(`[CircuitBreaker] Opened after ${this.failures} failures`);
        }
    }
    /**
     * Get circuit breaker status
     */
    getStatus() {
        return {
            state: this.state,
            failures: this.failures,
            lastFailure: this.lastFailureTime || undefined,
        };
    }
    /**
     * Manually reset circuit breaker
     */
    reset() {
        this.state = 'closed';
        this.failures = 0;
        this.lastFailureTime = 0;
    }
}
/**
 * Global resource tracker instance
 */
export const globalResourceTracker = new ResourceTracker();
/**
 * Default rate limiters
 */
export const rateLimiters = {
    // 100 inserts per second
    insert: new RateLimiter(100, 100),
    // 1000 searches per second
    search: new RateLimiter(1000, 1000),
    // 50 deletes per second
    delete: new RateLimiter(50, 50),
    // 10 batch operations per second
    batch: new RateLimiter(10, 10),
};
/**
 * Monitor and log resource usage
 */
export function logResourceUsage() {
    const stats = globalResourceTracker.getStats();
    console.log('[ResourceMonitor]', {
        memory: `${stats.memoryUsageMB.toFixed(2)}MB / ${stats.memoryLimitMB}MB (${stats.memoryUtilization.toFixed(1)}%)`,
        queries: stats.queryCount,
        avgQueryTime: `${stats.avgQueryTimeMs.toFixed(2)}ms`,
        qps: stats.queriesPerSecond.toFixed(2),
        uptime: `${stats.uptimeSeconds.toFixed(0)}s`,
    });
    // Warn if approaching limits
    if (stats.memoryUtilization > 80) {
        console.warn('[ResourceMonitor] WARNING: Memory usage above 80%');
    }
    if (stats.avgQueryTimeMs > SECURITY_LIMITS.QUERY_TIMEOUT_MS * 0.5) {
        console.warn('[ResourceMonitor] WARNING: Average query time approaching timeout');
    }
}
//# sourceMappingURL=limits.js.map