/**
 * AgentDB Re-exports for Backwards Compatibility
 *
 * This module provides backwards-compatible exports for code that previously
 * used embedded AgentDB controllers. Now proxies to agentdb npm package.
 *
 * @deprecated Import directly from specific agentdb paths for better tree-shaking
 * @since v1.7.0 - Integrated agentdb as proper dependency
 *
 * Example migration:
 * ```typescript
 * // Old (still works)
 * import { ReflexionMemory } from 'agentic-flow/agentdb';
 *
 * // New (recommended)
 * import { ReflexionMemory } from 'agentdb/controllers/ReflexionMemory';
 * ```
 */
// Import from agentdb package - using direct package imports for bundler moduleResolution
export { ReflexionMemory } from 'agentdb';
export { SkillLibrary } from 'agentdb';
export { EmbeddingService } from 'agentdb';
export { CausalMemoryGraph } from 'agentdb';
export { CausalRecall } from 'agentdb';
export { NightlyLearner } from 'agentdb';
export { ExplainableRecall } from 'agentdb';
// Note: These are custom types not exported from agentdb v1.3.9
// Users should import from agentdb directly if needed
// export type { LearningSystem } from 'agentdb/...';
// export type { ReasoningBank } from 'agentdb/...';
// Note: Optimizations not available in agentdb v1.3.9
// Users can implement custom optimizations or use AgentDB's built-in features
//# sourceMappingURL=index.js.map