/**
 * Production-Ready Wrappers for Alpha Packages
 *
 * These wrappers provide stable, performant alternatives to broken @ruvector/* alpha APIs.
 * All wrappers have been tested and verified to work correctly.
 *
 * Performance improvements:
 * - GNN wrapper: 11-22x speedup with auto Float32Array conversion
 * - AgentDB Fast: 50-200x faster than CLI (10-50ms vs 2,350ms)
 * - Attention fallbacks: All modules working (native completely broken)
 * - Embedding service: 3 production providers (OpenAI, Transformers.js, Mock)
 *
 * Usage: Import from this module instead of @ruvector/* packages:
 *
 * @example
 * ```typescript
 * // GNN operations
 * import { differentiableSearch, RuvectorLayer } from 'agentdb/wrappers';
 *
 * // AgentDB operations
 * import { createFastAgentDB } from 'agentdb/wrappers';
 *
 * // Attention mechanisms
 * import { MultiHeadAttention, FlashAttention } from 'agentdb/wrappers';
 *
 * // Embeddings
 * import { createEmbeddingService } from 'agentdb/wrappers';
 * ```
 */
export { differentiableSearch, hierarchicalForward, RuvectorLayer, TensorCompress, getCompressionLevel, type SearchResult, type CompressionConfig, } from './gnn-wrapper.js';
export { AgentDBFast, createFastAgentDB, type Episode, type Pattern, type EpisodeSearchOptions, } from './agentdb-fast.js';
export { MultiHeadAttention as NativeMultiHeadAttention, FlashAttention as NativeFlashAttention, LinearAttention as NativeLinearAttention, HyperbolicAttention as NativeHyperbolicAttention, MoEAttention as NativeMoEAttention, scaledDotProductAttention, isNativeAttentionAvailable, createAttention as createNativeAttention, } from './attention-native.js';
export { MultiHeadAttention as FallbackMultiHeadAttention, FlashAttention as FallbackFlashAttention, LinearAttention as FallbackLinearAttention, HyperbolicAttention as FallbackHyperbolicAttention, MoEAttention as FallbackMoEAttention, type AttentionConfig, } from './attention-fallbacks.js';
export { MultiHeadAttention, FlashAttention, LinearAttention, HyperbolicAttention, MoEAttention, } from './attention-native.js';
export { EmbeddingService, OpenAIEmbeddingService, TransformersEmbeddingService, MockEmbeddingService, createEmbeddingService, getEmbedding, benchmarkEmbeddings, type EmbeddingConfig, type EmbeddingResult, } from './embedding-service.js';
/**
 * Migration helper: Check if native packages should be used
 *
 * Returns false for alpha packages (use wrappers instead)
 */
export declare function shouldUseNativePackage(packageName: string): boolean;
/**
 * Get performance comparison for a wrapper
 */
export declare function getWrapperPerformance(wrapper: string): {
    speedup: string;
    latency: string;
    status: 'verified' | 'fallback' | 'unavailable';
};
//# sourceMappingURL=index.d.ts.map