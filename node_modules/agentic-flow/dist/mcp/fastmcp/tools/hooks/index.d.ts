/**
 * Hook Tools - Intelligent agent routing and self-learning
 * Integrates with ReasoningBank, LearningSystem, and Swarm
 *
 * NOW WITH FULL RUVECTOR INTELLIGENCE:
 * - @ruvector/sona: Micro-LoRA (~0.05ms), EWC++, Trajectory tracking
 * - @ruvector/attention: MoE, Flash, Hyperbolic, Graph attention
 * - ruvector core: HNSW indexing (150x faster)
 *
 * Available as BOTH:
 * 1. MCP Tools (via hooks-server.ts)
 * 2. CLI Hooks (via npx ruvector hooks)
 */
export { hookPreEditTool } from './pre-edit.js';
export { hookPostEditTool } from './post-edit.js';
export { hookPreCommandTool } from './pre-command.js';
export { hookPostCommandTool } from './post-command.js';
export { hookRouteTool } from './route.js';
export { hookExplainTool } from './explain.js';
export { hookPretrainTool } from './pretrain.js';
export { hookBuildAgentsTool } from './build-agents.js';
export { hookMetricsTool } from './metrics.js';
export { hookTransferTool } from './transfer.js';
export { getIntelligence, routeTaskIntelligent, beginTaskTrajectory, recordTrajectoryStep, endTaskTrajectory, storePattern, findSimilarPatterns, getIntelligenceStats, forceLearningCycle, computeAttentionSimilarity } from './intelligence-bridge.js';
export { intelligenceRouteTool, intelligenceTrajectoryStartTool, intelligenceTrajectoryStepTool, intelligenceTrajectoryEndTool, intelligencePatternStoreTool, intelligencePatternSearchTool, intelligenceStatsTool, intelligenceLearnTool, intelligenceAttentionTool, intelligenceTools } from './intelligence-tools.js';
export declare const hookTools: import("../../types/index.js").ToolDefinition[];
export declare const allHookTools: import("../../types/index.js").ToolDefinition[];
//# sourceMappingURL=index.d.ts.map