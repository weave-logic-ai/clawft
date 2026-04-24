/**
 * Advanced Memory System - Full Implementation for v1.7.1
 *
 * Provides high-level memory operations on top of HybridReasoningBank:
 * - Auto-consolidation (patterns → skills) using NightlyLearner
 * - Episodic replay (learn from failures)
 * - Causal reasoning (what-if analysis)
 * - Skill composition (combine learned skills)
 *
 * @example
 * ```typescript
 * import { AdvancedMemorySystem } from 'agentic-flow/reasoningbank';
 *
 * const memory = new AdvancedMemorySystem();
 *
 * // Auto-consolidate patterns into skills
 * const result = await memory.autoConsolidate({ minUses: 3, minSuccessRate: 0.7 });
 *
 * // Learn from failures
 * const failures = await memory.replayFailures('authentication', 5);
 *
 * // Causal what-if analysis
 * const insight = await memory.whatIfAnalysis('add caching');
 * ```
 */
export interface FailureAnalysis {
    critique: string;
    whatWentWrong: string[];
    howToFix: string[];
    similarFailures: number;
}
export interface SkillComposition {
    availableSkills: any[];
    compositionPlan: string;
    expectedSuccessRate: number;
}
export interface ConsolidationResult {
    skillsCreated: number;
    causalEdgesCreated: number;
    patternsAnalyzed: number;
    executionTimeMs: number;
    recommendations: string[];
}
export declare class AdvancedMemorySystem {
    private reasoning;
    private learner;
    private pool;
    constructor(options?: {
        preferWasm?: boolean;
    });
    /**
     * Auto-consolidate successful patterns into skills
     *
     * Uses NightlyLearner to:
     * 1. Discover causal edges from episode patterns
     * 2. Complete A/B experiments
     * 3. Calculate uplift for experiments
     * 4. Prune low-confidence edges
     * 5. Consolidate high-performing patterns into skills
     */
    autoConsolidate(options?: {
        minUses?: number;
        minSuccessRate?: number;
        lookbackDays?: number;
        dryRun?: boolean;
    }): Promise<ConsolidationResult>;
    /**
     * Learn from past failures with episodic replay
     *
     * Retrieves failed attempts, extracts lessons, and provides recommendations
     */
    replayFailures(task: string, k?: number): Promise<FailureAnalysis[]>;
    /**
     * Extract critique from failure pattern
     */
    private extractCritique;
    /**
     * Analyze what went wrong in a failure
     */
    private analyzeFailure;
    /**
     * Generate fix recommendations
     */
    private generateFixes;
    /**
     * What-if causal analysis
     *
     * Analyzes potential outcomes of taking an action based on causal evidence
     */
    whatIfAnalysis(action: string): Promise<{
        action: string;
        avgReward: number;
        avgUplift: number;
        confidence: number;
        evidenceCount: number;
        recommendation: 'DO_IT' | 'AVOID' | 'NEUTRAL';
        expectedImpact: string;
    }>;
    /**
     * Compose multiple skills for a complex task
     *
     * Finds relevant skills and creates an execution plan
     */
    composeSkills(task: string, k?: number): Promise<SkillComposition>;
    /**
     * Run automated learning cycle
     *
     * Discovers causal edges, consolidates skills, and optimizes performance
     */
    runLearningCycle(): Promise<ConsolidationResult>;
    /**
     * Get comprehensive memory statistics
     */
    getStats(): {
        reasoningBank: any;
        learner: string;
        memoryPool: any;
    };
}
//# sourceMappingURL=AdvancedMemory.d.ts.map