/**
 * Memory Distillation from trajectories
 * Algorithm 3 from ReasoningBank paper
 */
import type { Trajectory } from '../db/schema.js';
import type { Verdict } from './judge.js';
export interface DistilledMemory {
    title: string;
    description: string;
    content: string;
    tags: string[];
    domain?: string;
}
/**
 * Distill memories from a trajectory
 */
export declare function distillMemories(trajectory: Trajectory, verdict: Verdict, query: string, options?: {
    taskId?: string;
    agentId?: string;
    domain?: string;
}): Promise<string[]>;
//# sourceMappingURL=distill.d.ts.map