/**
 * Attention Controllers - Export Module
 *
 * Exports all attention mechanism controllers for memory systems:
 * - SelfAttentionController: Self-attention over memory entries
 * - CrossAttentionController: Cross-attention between query and contexts
 * - MultiHeadAttentionController: Multi-head attention with parallel heads
 */
export { SelfAttentionController } from './SelfAttentionController.js';
export { CrossAttentionController } from './CrossAttentionController.js';
export { MultiHeadAttentionController } from './MultiHeadAttentionController.js';
export type { SelfAttentionConfig, AttentionScore, SelfAttentionResult, MemoryEntry as SelfAttentionMemoryEntry } from './SelfAttentionController.js';
export type { CrossAttentionConfig, CrossAttentionScore, CrossAttentionResult, ContextEntry } from './CrossAttentionController.js';
export type { MultiHeadAttentionConfig, HeadAttentionOutput, MultiHeadAttentionResult, MemoryEntry as MultiHeadMemoryEntry } from './MultiHeadAttentionController.js';
//# sourceMappingURL=index.d.ts.map