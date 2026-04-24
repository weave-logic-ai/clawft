/**
 * ReasoningBank - Closed-loop memory system for AI agents
 * Based on arXiv:2509.25140 (Google DeepMind)
 *
 * @since v1.7.0 - Integrated AgentDB for optimal performance
 */
export { HybridReasoningBank } from './HybridBackend.js';
export { AdvancedMemorySystem } from './AdvancedMemory.js';
export type { PatternData, RetrievalOptions, CausalInsight } from './HybridBackend.js';
export type { FailureAnalysis, SkillComposition } from './AdvancedMemory.js';
export { ReflexionMemory } from 'agentdb/controllers/ReflexionMemory';
export { SkillLibrary } from 'agentdb/controllers/SkillLibrary';
export { CausalMemoryGraph } from 'agentdb/controllers/CausalMemoryGraph';
export { CausalRecall } from 'agentdb/controllers/CausalRecall';
export { NightlyLearner } from 'agentdb/controllers/NightlyLearner';
export { EmbeddingService } from 'agentdb/controllers/EmbeddingService';
export { retrieveMemories, formatMemoriesForPrompt } from './core/retrieve.js';
export type { RetrievedMemory } from './core/retrieve.js';
export { judgeTrajectory } from './core/judge.js';
export type { Verdict } from './core/judge.js';
export { distillMemories } from './core/distill.js';
export type { DistilledMemory } from './core/distill.js';
export { consolidate, shouldConsolidate } from './core/consolidate.js';
export type { ConsolidationResult } from './core/consolidate.js';
export { mattsParallel, mattsSequential } from './core/matts.js';
export type { MattsResult } from './core/matts.js';
export { computeEmbedding, clearEmbeddingCache } from './utils/embeddings.js';
export { mmrSelection, cosineSimilarity } from './utils/mmr.js';
export { scrubPII, containsPII, scrubMemory } from './utils/pii-scrubber.js';
export { loadConfig } from './utils/config.js';
import * as db from './db/queries.js';
export { db };
export type { ReasoningMemory, PatternEmbedding, PatternLink, TaskTrajectory, MattsRun, ConsolidationRun, Trajectory, TrajectoryStep } from './db/schema.js';
export declare function initialize(): Promise<void>;
export declare function runTask(options: {
    taskId: string;
    agentId: string;
    query: string;
    domain?: string;
    executeFn: (memories: any[]) => Promise<any>;
}): Promise<{
    verdict: any;
    usedMemories: any[];
    newMemories: string[];
    consolidated: boolean;
}>;
export declare const VERSION = "1.7.0";
export declare const PAPER_URL = "https://arxiv.org/html/2509.25140v1";
//# sourceMappingURL=index-new.d.ts.map