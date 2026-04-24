/**
 * Attention Type Definitions for AgentDB v2
 *
 * Shared types for @ruvector/attention integration across memory controllers.
 * These types are used by AttentionService and all enhanced controllers.
 *
 * @module types/attention
 * @see controllers/AttentionService
 * @see docs/integration/ARCHITECTURE.md
 */
// ============================================================================
// Type Guards
// ============================================================================
/**
 * Type guard for attention-enhanced causal edge
 */
export function isCausalEdgeWithAttention(edge) {
    return (typeof edge === 'object' &&
        edge !== null &&
        'fromMemoryId' in edge &&
        'toMemoryId' in edge &&
        ('hyperbolicScore' in edge || 'attentionWeight' in edge));
}
/**
 * Type guard for attention-enhanced pattern
 */
export function isPatternWithAttention(pattern) {
    return (typeof pattern === 'object' &&
        pattern !== null &&
        'taskType' in pattern &&
        ('flashScore' in pattern || 'expertId' in pattern));
}
/**
 * Type guard for attention-enhanced explanation
 */
export function isExplanationWithAttention(explanation) {
    return (typeof explanation === 'object' &&
        explanation !== null &&
        'nodes' in explanation &&
        'edges' in explanation &&
        ('hopDistances' in explanation || 'graphRoPEScores' in explanation));
}
//# sourceMappingURL=attention.js.map