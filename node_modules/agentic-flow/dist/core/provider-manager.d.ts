/**
 * Provider Manager - Intelligent LLM Provider Fallback & Dynamic Switching
 *
 * Features:
 * - Automatic failover between providers
 * - Health monitoring and circuit breaking
 * - Cost-based optimization
 * - Performance tracking
 * - Rate limit handling
 * - Provider quality scoring
 */
export type ProviderType = 'anthropic' | 'gemini' | 'openrouter' | 'onnx';
export interface ProviderConfig {
    name: ProviderType;
    apiKey?: string;
    baseUrl?: string;
    priority: number;
    maxRetries: number;
    timeout: number;
    costPerToken: number;
    enabled: boolean;
    healthCheckInterval?: number;
}
export interface ProviderHealth {
    provider: ProviderType;
    isHealthy: boolean;
    lastChecked: Date;
    consecutiveFailures: number;
    averageLatency: number;
    successRate: number;
    errorRate: number;
    circuitBreakerOpen: boolean;
}
export interface ProviderMetrics {
    provider: ProviderType;
    totalRequests: number;
    successfulRequests: number;
    failedRequests: number;
    totalTokens: number;
    totalCost: number;
    averageLatency: number;
    lastUsed: Date;
}
export interface FallbackStrategy {
    type: 'priority' | 'cost-optimized' | 'performance-optimized' | 'round-robin';
    maxFailures: number;
    recoveryTime: number;
    retryBackoff: 'linear' | 'exponential';
    costThreshold?: number;
    latencyThreshold?: number;
}
export declare class ProviderManager {
    private providers;
    private health;
    private metrics;
    private strategy;
    private currentProvider;
    private healthCheckIntervals;
    constructor(providers: ProviderConfig[], strategy?: FallbackStrategy);
    /**
     * Initialize provider configurations
     */
    private initializeProviders;
    /**
     * Start health check monitoring for all providers
     */
    private startHealthChecks;
    /**
     * Perform health check on a provider
     */
    private performHealthCheck;
    /**
     * Select the best provider based on strategy
     */
    selectProvider(taskComplexity?: 'simple' | 'medium' | 'complex', estimatedTokens?: number): Promise<ProviderType>;
    /**
     * Get list of healthy, available providers
     */
    private getAvailableProviders;
    /**
     * Select provider by priority
     */
    private selectByPriority;
    /**
     * Select provider by cost optimization
     */
    private selectByCost;
    /**
     * Select provider by performance (latency + success rate)
     */
    private selectByPerformance;
    /**
     * Select provider using round-robin
     */
    private selectRoundRobin;
    /**
     * Execute request with automatic fallback
     */
    executeWithFallback<T>(requestFn: (provider: ProviderType) => Promise<T>, taskComplexity?: 'simple' | 'medium' | 'complex', estimatedTokens?: number): Promise<{
        result: T;
        provider: ProviderType;
        attempts: number;
    }>;
    /**
     * Execute request with retry logic
     */
    private executeWithRetry;
    /**
     * Calculate backoff delay
     */
    private calculateBackoffDelay;
    /**
     * Check if error is retryable
     */
    private isRetryableError;
    /**
     * Record successful request
     */
    private recordSuccess;
    /**
     * Record failed request
     */
    private recordFailure;
    /**
     * Get current metrics for all providers
     */
    getMetrics(): ProviderMetrics[];
    /**
     * Get health status for all providers
     */
    getHealth(): ProviderHealth[];
    /**
     * Get cost summary
     */
    getCostSummary(): {
        total: number;
        byProvider: Record<ProviderType, number>;
        totalTokens: number;
    };
    /**
     * Cleanup and stop health checks
     */
    destroy(): void;
}
//# sourceMappingURL=provider-manager.d.ts.map