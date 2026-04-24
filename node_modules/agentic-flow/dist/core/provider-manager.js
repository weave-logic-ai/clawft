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
import { logger } from '../utils/logger.js';
export class ProviderManager {
    providers = new Map();
    health = new Map();
    metrics = new Map();
    strategy;
    currentProvider = null;
    healthCheckIntervals = new Map();
    constructor(providers, strategy = {
        type: 'priority',
        maxFailures: 3,
        recoveryTime: 60000, // 1 minute
        retryBackoff: 'exponential'
    }) {
        this.strategy = strategy;
        this.initializeProviders(providers);
        this.startHealthChecks();
    }
    /**
     * Initialize provider configurations
     */
    initializeProviders(providers) {
        for (const provider of providers) {
            this.providers.set(provider.name, provider);
            // Initialize health tracking
            this.health.set(provider.name, {
                provider: provider.name,
                isHealthy: true,
                lastChecked: new Date(),
                consecutiveFailures: 0,
                averageLatency: 0,
                successRate: 1.0,
                errorRate: 0.0,
                circuitBreakerOpen: false
            });
            // Initialize metrics
            this.metrics.set(provider.name, {
                provider: provider.name,
                totalRequests: 0,
                successfulRequests: 0,
                failedRequests: 0,
                totalTokens: 0,
                totalCost: 0,
                averageLatency: 0,
                lastUsed: new Date()
            });
        }
        logger.info('Provider Manager initialized', {
            providers: Array.from(this.providers.keys()),
            strategy: this.strategy.type
        });
    }
    /**
     * Start health check monitoring for all providers
     */
    startHealthChecks() {
        for (const [name, config] of this.providers.entries()) {
            if (config.healthCheckInterval) {
                const interval = setInterval(() => {
                    this.performHealthCheck(name);
                }, config.healthCheckInterval);
                this.healthCheckIntervals.set(name, interval);
            }
        }
    }
    /**
     * Perform health check on a provider
     */
    async performHealthCheck(provider) {
        const health = this.health.get(provider);
        if (!health)
            return;
        try {
            const startTime = Date.now();
            // TODO: Implement actual health check API call
            // For now, just check if circuit breaker should recover
            const latency = Date.now() - startTime;
            health.lastChecked = new Date();
            health.averageLatency = (health.averageLatency + latency) / 2;
            // Recovery logic: reset circuit breaker after recovery time
            if (health.circuitBreakerOpen) {
                const timeSinceLastCheck = Date.now() - health.lastChecked.getTime();
                if (timeSinceLastCheck >= this.strategy.recoveryTime) {
                    logger.info('Circuit breaker recovering', { provider });
                    health.circuitBreakerOpen = false;
                    health.consecutiveFailures = 0;
                }
            }
            logger.debug('Health check completed', { provider, health });
        }
        catch (error) {
            logger.error('Health check failed', { provider, error: error.message });
            this.recordFailure(provider, error);
        }
    }
    /**
     * Select the best provider based on strategy
     */
    async selectProvider(taskComplexity, estimatedTokens) {
        const availableProviders = this.getAvailableProviders();
        if (availableProviders.length === 0) {
            throw new Error('No healthy providers available');
        }
        let selectedProvider;
        switch (this.strategy.type) {
            case 'priority':
                selectedProvider = this.selectByPriority(availableProviders);
                break;
            case 'cost-optimized':
                selectedProvider = this.selectByCost(availableProviders, estimatedTokens);
                break;
            case 'performance-optimized':
                selectedProvider = this.selectByPerformance(availableProviders);
                break;
            case 'round-robin':
                selectedProvider = this.selectRoundRobin(availableProviders);
                break;
            default:
                selectedProvider = availableProviders[0];
        }
        // Apply task complexity heuristics
        if (taskComplexity === 'complex' && this.providers.has('anthropic')) {
            const anthropicHealth = this.health.get('anthropic');
            if (anthropicHealth?.isHealthy && !anthropicHealth.circuitBreakerOpen) {
                selectedProvider = 'anthropic'; // Prefer Claude for complex tasks
            }
        }
        else if (taskComplexity === 'simple' && this.providers.has('gemini')) {
            const geminiHealth = this.health.get('gemini');
            if (geminiHealth?.isHealthy && !geminiHealth.circuitBreakerOpen) {
                selectedProvider = 'gemini'; // Prefer Gemini for simple tasks (faster, cheaper)
            }
        }
        this.currentProvider = selectedProvider;
        logger.info('Provider selected', {
            provider: selectedProvider,
            strategy: this.strategy.type,
            taskComplexity,
            estimatedTokens
        });
        return selectedProvider;
    }
    /**
     * Get list of healthy, available providers
     */
    getAvailableProviders() {
        const available = [];
        for (const [name, config] of this.providers.entries()) {
            const health = this.health.get(name);
            if (config.enabled &&
                health?.isHealthy &&
                !health.circuitBreakerOpen) {
                available.push(name);
            }
        }
        // Sort by priority
        available.sort((a, b) => {
            const priorityA = this.providers.get(a)?.priority ?? 999;
            const priorityB = this.providers.get(b)?.priority ?? 999;
            return priorityA - priorityB;
        });
        return available;
    }
    /**
     * Select provider by priority
     */
    selectByPriority(providers) {
        return providers[0]; // Already sorted by priority
    }
    /**
     * Select provider by cost optimization
     */
    selectByCost(providers, estimatedTokens) {
        if (!estimatedTokens) {
            return this.selectByPriority(providers);
        }
        let bestProvider = providers[0];
        let lowestCost = Infinity;
        for (const provider of providers) {
            const config = this.providers.get(provider);
            if (!config)
                continue;
            const estimatedCost = (estimatedTokens / 1_000_000) * config.costPerToken;
            if (estimatedCost < lowestCost) {
                lowestCost = estimatedCost;
                bestProvider = provider;
            }
        }
        return bestProvider;
    }
    /**
     * Select provider by performance (latency + success rate)
     */
    selectByPerformance(providers) {
        let bestProvider = providers[0];
        let bestScore = -Infinity;
        for (const provider of providers) {
            const health = this.health.get(provider);
            if (!health)
                continue;
            // Score = success rate (weighted 70%) - normalized latency (weighted 30%)
            const normalizedLatency = health.averageLatency / 1000; // Convert to seconds
            const score = (health.successRate * 0.7) - (normalizedLatency * 0.3);
            if (score > bestScore) {
                bestScore = score;
                bestProvider = provider;
            }
        }
        return bestProvider;
    }
    /**
     * Select provider using round-robin
     */
    selectRoundRobin(providers) {
        if (!this.currentProvider || !providers.includes(this.currentProvider)) {
            return providers[0];
        }
        const currentIndex = providers.indexOf(this.currentProvider);
        const nextIndex = (currentIndex + 1) % providers.length;
        return providers[nextIndex];
    }
    /**
     * Execute request with automatic fallback
     */
    async executeWithFallback(requestFn, taskComplexity, estimatedTokens) {
        const startTime = Date.now();
        let attempts = 0;
        let lastError = null;
        // Try each available provider
        const availableProviders = this.getAvailableProviders();
        const provider = await this.selectProvider(taskComplexity, estimatedTokens);
        // Start with selected provider, then try fallbacks
        const providersToTry = [
            provider,
            ...availableProviders.filter(p => p !== provider)
        ];
        for (const currentProvider of providersToTry) {
            attempts++;
            const config = this.providers.get(currentProvider);
            if (!config)
                continue;
            try {
                logger.info('Attempting request', { provider: currentProvider, attempt: attempts });
                const requestStartTime = Date.now();
                const result = await this.executeWithRetry(requestFn, currentProvider, config.maxRetries);
                const latency = Date.now() - requestStartTime;
                // Record success
                this.recordSuccess(currentProvider, latency, estimatedTokens || 0);
                logger.info('Request successful', {
                    provider: currentProvider,
                    attempts,
                    latency,
                    totalTime: Date.now() - startTime
                });
                return { result, provider: currentProvider, attempts };
            }
            catch (error) {
                lastError = error;
                logger.warn('Provider request failed', {
                    provider: currentProvider,
                    attempt: attempts,
                    error: lastError.message
                });
                // Record failure
                this.recordFailure(currentProvider, lastError);
                // Continue to next provider
                continue;
            }
        }
        // All providers failed
        throw new Error(`All providers failed after ${attempts} attempts. Last error: ${lastError?.message}`);
    }
    /**
     * Execute request with retry logic
     */
    async executeWithRetry(requestFn, provider, maxRetries) {
        let lastError = null;
        for (let retry = 0; retry <= maxRetries; retry++) {
            try {
                // Apply backoff delay
                if (retry > 0) {
                    const delay = this.calculateBackoffDelay(retry);
                    logger.debug('Retry backoff', { provider, retry, delay });
                    await new Promise(resolve => setTimeout(resolve, delay));
                }
                return await requestFn(provider);
            }
            catch (error) {
                lastError = error;
                // Check if error is retryable
                if (!this.isRetryableError(error)) {
                    throw error;
                }
                logger.debug('Retryable error, continuing', {
                    provider,
                    retry,
                    error: lastError.message
                });
            }
        }
        throw lastError || new Error('Max retries exceeded');
    }
    /**
     * Calculate backoff delay
     */
    calculateBackoffDelay(retry) {
        if (this.strategy.retryBackoff === 'exponential') {
            // Exponential backoff: 1s, 2s, 4s, 8s, 16s...
            return Math.min(1000 * Math.pow(2, retry), 30000); // Max 30s
        }
        else {
            // Linear backoff: 1s, 2s, 3s, 4s...
            return Math.min(1000 * retry, 10000); // Max 10s
        }
    }
    /**
     * Check if error is retryable
     */
    isRetryableError(error) {
        const retryablePatterns = [
            /rate limit/i,
            /timeout/i,
            /connection/i,
            /network/i,
            /503/,
            /502/,
            /429/
        ];
        return retryablePatterns.some(pattern => pattern.test(error.message));
    }
    /**
     * Record successful request
     */
    recordSuccess(provider, latency, tokens) {
        const metrics = this.metrics.get(provider);
        const health = this.health.get(provider);
        const config = this.providers.get(provider);
        if (metrics) {
            metrics.totalRequests++;
            metrics.successfulRequests++;
            metrics.totalTokens += tokens;
            metrics.totalCost += (tokens / 1_000_000) * (config?.costPerToken || 0);
            metrics.averageLatency = (metrics.averageLatency + latency) / 2;
            metrics.lastUsed = new Date();
        }
        if (health) {
            health.consecutiveFailures = 0;
            health.isHealthy = true;
            health.circuitBreakerOpen = false;
            health.successRate = metrics ? metrics.successfulRequests / metrics.totalRequests : 1.0;
            health.errorRate = 1.0 - health.successRate;
            health.averageLatency = latency;
        }
    }
    /**
     * Record failed request
     */
    recordFailure(provider, error) {
        const metrics = this.metrics.get(provider);
        const health = this.health.get(provider);
        if (metrics) {
            metrics.totalRequests++;
            metrics.failedRequests++;
        }
        if (health) {
            health.consecutiveFailures++;
            health.successRate = metrics ? metrics.successfulRequests / metrics.totalRequests : 0.0;
            health.errorRate = 1.0 - health.successRate;
            // Open circuit breaker if threshold exceeded
            if (health.consecutiveFailures >= this.strategy.maxFailures) {
                logger.warn('Circuit breaker opened', {
                    provider,
                    consecutiveFailures: health.consecutiveFailures,
                    threshold: this.strategy.maxFailures
                });
                health.circuitBreakerOpen = true;
                health.isHealthy = false;
            }
        }
        logger.error('Provider failure recorded', {
            provider,
            error: error.message,
            consecutiveFailures: health?.consecutiveFailures,
            circuitBreakerOpen: health?.circuitBreakerOpen
        });
    }
    /**
     * Get current metrics for all providers
     */
    getMetrics() {
        return Array.from(this.metrics.values());
    }
    /**
     * Get health status for all providers
     */
    getHealth() {
        return Array.from(this.health.values());
    }
    /**
     * Get cost summary
     */
    getCostSummary() {
        let total = 0;
        let totalTokens = 0;
        const byProvider = {};
        for (const [name, metrics] of this.metrics.entries()) {
            total += metrics.totalCost;
            totalTokens += metrics.totalTokens;
            byProvider[name] = metrics.totalCost;
        }
        return { total, byProvider: byProvider, totalTokens };
    }
    /**
     * Cleanup and stop health checks
     */
    destroy() {
        for (const interval of this.healthCheckIntervals.values()) {
            clearInterval(interval);
        }
        this.healthCheckIntervals.clear();
        logger.info('Provider Manager destroyed');
    }
}
//# sourceMappingURL=provider-manager.js.map