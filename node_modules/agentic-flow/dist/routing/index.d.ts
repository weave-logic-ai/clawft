/**
 * Routing Module
 *
 * Exports:
 * - CircuitBreakerRouter: Fault-tolerant agent routing
 * - SemanticRouter: HNSW-powered intent matching
 */
export { CircuitBreakerRouter, CircuitState } from './CircuitBreakerRouter.js';
export type { CircuitBreakerConfig, RouteRequest, RouteResult, AgentHealth, } from './CircuitBreakerRouter.js';
export { SemanticRouter } from './SemanticRouter.js';
export type { AgentIntent, RoutingResult, MultiIntentResult, } from './SemanticRouter.js';
//# sourceMappingURL=index.d.ts.map