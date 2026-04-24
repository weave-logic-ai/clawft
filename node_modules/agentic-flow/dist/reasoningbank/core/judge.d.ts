/**
 * LLM-as-Judge for trajectory evaluation
 * Algorithm 2 from ReasoningBank paper
 */
import type { Trajectory } from '../db/schema.js';
export interface Verdict {
    label: 'Success' | 'Failure';
    confidence: number;
    reasons: string[];
}
/**
 * Judge a task trajectory using LLM evaluation
 */
export declare function judgeTrajectory(trajectory: Trajectory, query: string, options?: {
    useCache?: boolean;
}): Promise<Verdict>;
//# sourceMappingURL=judge.d.ts.map