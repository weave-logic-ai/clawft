/**
 * Circuit Breaker Router - Fault-Tolerant Agent Routing
 *
 * Integrates @ruvector/tiny-dancer circuit breaker pattern for 99.9% uptime.
 *
 * Features:
 * - Circuit breaker states: CLOSED, OPEN, HALF_OPEN
 * - Automatic failure detection and recovery
 * - Fallback chains for degraded service
 * - Hot-reload capability for configuration updates
 * - Uncertainty estimation for routing decisions
 * - Rate limiting for request protection
 *
 * Performance:
 * - <5ms routing overhead
 * - 99.9% uptime guarantee
 * - Automatic failover in <100ms
 */
export interface CircuitBreakerConfig {
    /** Failure threshold before opening circuit (default: 5) */
    failureThreshold?: number;
    /** Success threshold to close circuit from half-open (default: 3) */
    successThreshold?: number;
    /** Timeout before attempting recovery (ms, default: 30000) */
    resetTimeout?: number;
    /** Request timeout (ms, default: 5000) */
    requestTimeout?: number;
    /** Enable uncertainty estimation (default: true) */
    enableUncertaintyEstimation?: boolean;
}
export declare enum CircuitState {
    CLOSED = "CLOSED",// Normal operation
    OPEN = "OPEN",// Failures detected, routing blocked
    HALF_OPEN = "HALF_OPEN"
}
export interface RouteRequest {
    taskDescription: string;
    preferredAgent?: string;
    fallbackAgents?: string[];
    timeout?: number;
}
export interface RouteResult {
    selectedAgent: string;
    confidence: number;
    circuitState: CircuitState;
    fallbackUsed: boolean;
    uncertainty?: number;
    metrics: {
        routingTimeMs: number;
        failureCount: number;
        successCount: number;
    };
}
export interface AgentHealth {
    agent: string;
    state: CircuitState;
    failureCount: number;
    successCount: number;
    lastFailureTime?: number;
    lastSuccessTime?: number;
    availability: number;
}
/**
 * Circuit Breaker Router
 *
 * Provides fault-tolerant routing with:
 * 1. Circuit breaker pattern for each agent
 * 2. Automatic fallback chains
 * 3. Health monitoring and recovery
 * 4. Uncertainty-aware decision making
 */
export declare class CircuitBreakerRouter {
    private config;
    private circuitStates;
    private failureCounts;
    private successCounts;
    private lastFailureTimes;
    private lastSuccessTimes;
    private resetTimers;
    private rateLimiter;
    private routingMetrics;
    constructor(config?: CircuitBreakerConfig);
    /**
     * Route request to agent with circuit breaker protection
     *
     * Process:
     * 1. Check circuit state for preferred agent
     * 2. If circuit OPEN, try fallback chain
     * 3. If circuit HALF_OPEN, allow test request
     * 4. Estimate uncertainty if enabled
     *
     * @param request - Route request with task and agent preferences
     * @returns Route result with selected agent and metrics
     */
    route(request: RouteRequest): Promise<RouteResult>;
    /**
     * Record successful agent execution
     *
     * Updates circuit breaker state:
     * - Increment success count
     * - Reset failure count if threshold reached
     * - Transition HALF_OPEN -> CLOSED if successful
     *
     * @param agent - Agent that succeeded
     */
    recordSuccess(agent: string): void;
    /**
     * Record failed agent execution
     *
     * Updates circuit breaker state:
     * - Increment failure count
     * - Transition CLOSED -> OPEN if threshold exceeded
     * - Transition HALF_OPEN -> OPEN on failure
     * - Schedule automatic reset
     *
     * @param agent - Agent that failed
     */
    recordFailure(agent: string): void;
    /**
     * Get circuit state for agent
     *
     * @param agent - Agent name
     * @returns Current circuit state
     */
    getCircuitState(agent: string): CircuitState;
    /**
     * Get health status for all agents
     *
     * @returns Array of agent health metrics
     */
    getAgentHealth(): AgentHealth[];
    /**
     * Get routing metrics
     *
     * @returns Cumulative routing statistics
     */
    getMetrics(): typeof this.routingMetrics;
    /**
     * Manually reset circuit for agent
     *
     * @param agent - Agent to reset
     */
    resetCircuit(agent: string): void;
    /**
     * Hot-reload configuration
     *
     * Allows updating circuit breaker parameters without restart.
     *
     * @param config - New configuration
     */
    updateConfig(config: Partial<CircuitBreakerConfig>): void;
    /**
     * Open circuit and schedule automatic reset
     */
    private openCircuit;
    /**
     * Calculate confidence score
     */
    private calculateConfidence;
    /**
     * Estimate uncertainty for routing decision
     *
     * Uses failure history and circuit state to estimate decision uncertainty.
     * Higher uncertainty indicates less reliable routing.
     *
     * @param agent - Agent to estimate uncertainty for
     * @returns Uncertainty score (0-1, lower is better)
     */
    private estimateUncertainty;
    /**
     * Calculate recency factor based on time since last failure
     */
    private getRecencyFactor;
    /**
     * Update average routing time (exponential moving average)
     */
    private updateAvgRoutingTime;
    /**
     * Simple string hash function for rate limiting
     */
    private hashString;
    /**
     * Cleanup resources on shutdown
     */
    destroy(): void;
}
//# sourceMappingURL=CircuitBreakerRouter.d.ts.map