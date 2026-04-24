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
import { HybridReasoningBank } from './HybridBackend.js';
import { NightlyLearner } from 'agentdb/controllers/NightlyLearner';
import { SharedMemoryPool } from '../memory/SharedMemoryPool.js';
export class AdvancedMemorySystem {
    reasoning;
    learner;
    pool;
    constructor(options = {}) {
        this.reasoning = new HybridReasoningBank(options);
        this.pool = SharedMemoryPool.getInstance();
        const db = this.pool.getDatabase();
        const embedder = this.pool.getEmbedder();
        // Initialize NightlyLearner with optimized config
        this.learner = new NightlyLearner(db, embedder, {
            minSimilarity: 0.7,
            minSampleSize: 5,
            confidenceThreshold: 0.6,
            upliftThreshold: 0.1,
            pruneOldEdges: true,
            edgeMaxAgeDays: 90,
            autoExperiments: true,
            experimentBudget: 100
        });
    }
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
    async autoConsolidate(options = {}) {
        const startTime = Date.now();
        try {
            // Run NightlyLearner's discovery and consolidation pipeline
            const report = await this.learner.run();
            // Also run skill consolidation from HybridReasoningBank
            const skillResult = await this.reasoning.autoConsolidate(options.minUses || 3, options.minSuccessRate || 0.7, options.lookbackDays || 30);
            return {
                skillsCreated: skillResult.skillsCreated + (report.edgesDiscovered || 0),
                causalEdgesCreated: report.edgesDiscovered || 0,
                patternsAnalyzed: report.experimentsCompleted || 0,
                executionTimeMs: Date.now() - startTime,
                recommendations: report.recommendations || []
            };
        }
        catch (error) {
            console.error('[AdvancedMemorySystem] Auto-consolidation failed:', error);
            // Fallback to basic consolidation
            const skillResult = await this.reasoning.autoConsolidate(options.minUses || 3, options.minSuccessRate || 0.7, options.lookbackDays || 30);
            return {
                skillsCreated: skillResult.skillsCreated,
                causalEdgesCreated: 0,
                patternsAnalyzed: 0,
                executionTimeMs: Date.now() - startTime,
                recommendations: ['Causal discovery unavailable - basic consolidation completed']
            };
        }
    }
    /**
     * Learn from past failures with episodic replay
     *
     * Retrieves failed attempts, extracts lessons, and provides recommendations
     */
    async replayFailures(task, k = 5) {
        const failures = await this.reasoning.retrievePatterns(task, {
            k,
            onlyFailures: true
        });
        return failures.map(f => ({
            critique: f.critique || this.extractCritique(f),
            whatWentWrong: this.analyzeFailure(f),
            howToFix: this.generateFixes(f),
            similarFailures: failures.length
        }));
    }
    /**
     * Extract critique from failure pattern
     */
    extractCritique(failure) {
        if (failure.critique)
            return failure.critique;
        if (failure.task)
            return `Failed at: ${failure.task}`;
        return 'No critique available';
    }
    /**
     * Analyze what went wrong in a failure
     */
    analyzeFailure(failure) {
        const issues = [];
        if (failure.reward !== undefined && failure.reward < 0.3) {
            issues.push('Low success rate observed');
        }
        if (failure.latencyMs && failure.latencyMs > 5000) {
            issues.push('High latency detected');
        }
        if (failure.task) {
            issues.push(`Task type: ${failure.task}`);
        }
        if (issues.length === 0) {
            issues.push('General failure - review approach');
        }
        return issues;
    }
    /**
     * Generate fix recommendations
     */
    generateFixes(failure) {
        const fixes = [];
        // Look for successful patterns with similar tasks
        fixes.push('Review similar successful patterns');
        if (failure.latencyMs && failure.latencyMs > 5000) {
            fixes.push('Optimize for lower latency');
        }
        if (failure.reward !== undefined && failure.reward < 0.3) {
            fixes.push('Consider alternative approach');
        }
        fixes.push('Add more validation and error handling');
        return fixes;
    }
    /**
     * What-if causal analysis
     *
     * Analyzes potential outcomes of taking an action based on causal evidence
     */
    async whatIfAnalysis(action) {
        const causalInsight = await this.reasoning.whatIfAnalysis(action);
        // Generate impact description
        let expectedImpact = '';
        if (causalInsight.avgUplift > 0.2) {
            expectedImpact = `Highly beneficial: Expected +${(causalInsight.avgUplift * 100).toFixed(1)}% improvement`;
        }
        else if (causalInsight.avgUplift > 0.1) {
            expectedImpact = `Beneficial: Expected +${(causalInsight.avgUplift * 100).toFixed(1)}% improvement`;
        }
        else if (causalInsight.avgUplift > 0) {
            expectedImpact = `Slightly positive: Expected +${(causalInsight.avgUplift * 100).toFixed(1)}% improvement`;
        }
        else if (causalInsight.avgUplift < -0.1) {
            expectedImpact = `Harmful: Expected ${(causalInsight.avgUplift * 100).toFixed(1)}% degradation`;
        }
        else {
            expectedImpact = 'Neutral or insufficient evidence';
        }
        return {
            ...causalInsight,
            expectedImpact
        };
    }
    /**
     * Compose multiple skills for a complex task
     *
     * Finds relevant skills and creates an execution plan
     */
    async composeSkills(task, k = 5) {
        const skills = await this.reasoning.searchSkills(task, k);
        // Sort by success rate and usage
        const sortedSkills = skills.sort((a, b) => {
            const scoreA = (a.successRate || 0) * 0.7 + (Math.log(a.uses || 1) / 10) * 0.3;
            const scoreB = (b.successRate || 0) * 0.7 + (Math.log(b.uses || 1) / 10) * 0.3;
            return scoreB - scoreA;
        });
        // Create composition plan
        let compositionPlan = '';
        if (sortedSkills.length === 0) {
            compositionPlan = 'No relevant skills found';
        }
        else if (sortedSkills.length === 1) {
            compositionPlan = sortedSkills[0].name;
        }
        else {
            compositionPlan = sortedSkills.slice(0, 3).map(s => s.name).join(' → ');
        }
        // Calculate expected success rate (weighted average)
        let expectedSuccessRate = 0;
        if (sortedSkills.length > 0) {
            const weights = sortedSkills.map(s => s.uses || 1);
            const totalWeight = weights.reduce((sum, w) => sum + w, 0);
            expectedSuccessRate = sortedSkills.reduce((sum, s, i) => sum + (s.successRate || 0) * weights[i] / totalWeight, 0);
        }
        return {
            availableSkills: sortedSkills,
            compositionPlan,
            expectedSuccessRate
        };
    }
    /**
     * Run automated learning cycle
     *
     * Discovers causal edges, consolidates skills, and optimizes performance
     */
    async runLearningCycle() {
        return this.autoConsolidate({
            minUses: 3,
            minSuccessRate: 0.7,
            lookbackDays: 30,
            dryRun: false
        });
    }
    /**
     * Get comprehensive memory statistics
     */
    getStats() {
        return {
            reasoningBank: this.reasoning.getStats(),
            learner: 'NightlyLearner configured with auto-experiments',
            memoryPool: this.pool.getStats()
        };
    }
}
//# sourceMappingURL=AdvancedMemory.js.map