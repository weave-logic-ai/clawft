/**
 * Intelligence MCP Tools - Expose RuVector Intelligence via MCP
 *
 * These tools provide direct access to the full RuVector ecosystem:
 * - @ruvector/sona: Micro-LoRA (~0.05ms), EWC++, Trajectory tracking
 * - @ruvector/attention: MoE, Flash, Hyperbolic, Graph attention
 * - ruvector core: HNSW indexing (150x faster than brute force)
 *
 * Available both as MCP tools AND CLI hooks.
 */
import type { ToolDefinition } from '../../types/index.js';
/**
 * Intelligence Route Tool
 * Route tasks using SONA Micro-LoRA + MoE Attention + HNSW indexing
 */
export declare const intelligenceRouteTool: ToolDefinition;
/**
 * Trajectory Start Tool
 * Begin trajectory tracking for learning from task execution
 */
export declare const intelligenceTrajectoryStartTool: ToolDefinition;
/**
 * Trajectory Step Tool
 * Record intermediate steps during task execution
 */
export declare const intelligenceTrajectoryStepTool: ToolDefinition;
/**
 * Trajectory End Tool
 * Complete trajectory and trigger learning with EWC++
 */
export declare const intelligenceTrajectoryEndTool: ToolDefinition;
/**
 * Pattern Store Tool
 * Store successful patterns in ReasoningBank for future retrieval
 */
export declare const intelligencePatternStoreTool: ToolDefinition;
/**
 * Pattern Search Tool
 * Find similar patterns using HNSW (150x faster than brute force)
 */
export declare const intelligencePatternSearchTool: ToolDefinition;
/**
 * Intelligence Stats Tool
 * Get statistics about the intelligence layer
 */
export declare const intelligenceStatsTool: ToolDefinition;
/**
 * Force Learning Tool
 * Trigger an immediate learning cycle
 */
export declare const intelligenceLearnTool: ToolDefinition;
/**
 * Attention Compute Tool
 * Compute attention-weighted similarity using MoE/Flash/Hyperbolic
 */
export declare const intelligenceAttentionTool: ToolDefinition;
export declare const intelligenceTools: ToolDefinition[];
//# sourceMappingURL=intelligence-tools.d.ts.map