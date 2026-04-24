/**
 * MaTTS: Memory-aware Test-Time Scaling
 * Algorithm 5 from ReasoningBank paper
 *
 * Two modes:
 * - Parallel: k independent rollouts with self-contrast aggregation
 * - Sequential: r iterative refinements with check-and-correct
 */
import type { Trajectory } from '../db/schema.js';
export interface MattsResult {
    runId: string;
    mode: 'parallel' | 'sequential';
    k: number;
    trajectories: Array<{
        id: string;
        verdict: {
            label: string;
            confidence: number;
        };
        trajectory: Trajectory;
    }>;
    aggregatedMemories: string[];
    successRate: number;
    duration: number;
}
/**
 * Run MaTTS in parallel mode
 * Execute k independent rollouts and aggregate via self-contrast
 */
export declare function mattsParallel(taskFn: () => Promise<Trajectory>, query: string, options?: {
    k?: number;
    taskId?: string;
    agentId?: string;
    domain?: string;
}): Promise<MattsResult>;
/**
 * Run MaTTS in sequential mode
 * Iterative refinement with check-and-correct
 */
export declare function mattsSequential(taskFn: (memories: any[]) => Promise<Trajectory>, query: string, options?: {
    r?: number;
    taskId?: string;
    agentId?: string;
    domain?: string;
}): Promise<MattsResult>;
//# sourceMappingURL=matts.d.ts.map