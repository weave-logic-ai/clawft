/**
 * Pre-Edit Hook - Context retrieval and agent routing before file edit
 * Latency target: <20ms
 *
 * NOW WITH RUVECTOR INTELLIGENCE:
 * - Trajectory tracking for learning from edit sequences
 * - HNSW-powered similar file retrieval
 * - SONA pattern matching for agent selection
 */
import type { ToolDefinition } from '../../types/index.js';
declare const activeEditTrajectories: Map<string, number>;
export { activeEditTrajectories };
export declare const hookPreEditTool: ToolDefinition;
//# sourceMappingURL=pre-edit.d.ts.map