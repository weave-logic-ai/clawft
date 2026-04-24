/**
 * Hybrid ReasoningBank Backend - Full Implementation for v1.7.1
 *
 * Combines Rust WASM (compute) + AgentDB TypeScript (storage) for optimal performance:
 * - WASM: 10x faster similarity computation
 * - AgentDB: Persistent SQLite storage with frontier memory
 * - CausalRecall: Utility-based reranking with causal uplift
 * - Automatic backend selection based on task requirements
 *
 * @example
 * ```typescript
 * import { HybridReasoningBank } from 'agentic-flow/reasoningbank';
 *
 * const rb = new HybridReasoningBank({ preferWasm: true });
 * await rb.storePattern({ task: '...', success: true, reward: 0.95 });
 * const patterns = await rb.retrievePatterns('similar task', { k: 5 });
 * const strategy = await rb.learnStrategy('API optimization');
 * ```
 */
export interface PatternData {
    sessionId: string;
    task: string;
    input?: string;
    output?: string;
    critique?: string;
    success: boolean;
    reward: number;
    latencyMs?: number;
    tokensUsed?: number;
}
export interface RetrievalOptions {
    k?: number;
    minReward?: number;
    onlySuccesses?: boolean;
    onlyFailures?: boolean;
}
export interface CausalInsight {
    action: string;
    avgReward: number;
    avgUplift: number;
    confidence: number;
    evidenceCount: number;
    recommendation: 'DO_IT' | 'AVOID' | 'NEUTRAL';
}
export declare class HybridReasoningBank {
    private memory;
    private reflexion;
    private skills;
    private causalRecall;
    private causalGraph;
    private useWasm;
    private wasmModule;
    constructor(options?: {
        preferWasm?: boolean;
    });
    private loadWasmModule;
    /**
     * Store a reasoning pattern
     */
    storePattern(pattern: PatternData): Promise<number>;
    /**
     * Retrieve similar patterns with optional WASM acceleration
     */
    retrievePatterns(query: string, options?: RetrievalOptions): Promise<any[]>;
    /**
     * Learn optimal strategy for a task
     *
     * Combines pattern retrieval with causal analysis to provide evidence-based recommendations
     */
    learnStrategy(task: string): Promise<{
        patterns: any[];
        causality: CausalInsight;
        confidence: number;
        recommendation: string;
    }>;
    /**
     * Auto-consolidate patterns into skills
     */
    autoConsolidate(minUses?: number, minSuccessRate?: number, lookbackDays?: number): Promise<{
        skillsCreated: number;
    }>;
    /**
     * What-if causal analysis
     */
    whatIfAnalysis(action: string): Promise<CausalInsight>;
    /**
     * Search for relevant skills
     */
    searchSkills(taskType: string, k?: number): Promise<any[]>;
    /**
     * Get statistics
     */
    getStats(): {
        causalRecall: any;
        reflexion: any;
        skills: number;
    };
}
//# sourceMappingURL=HybridBackend.d.ts.map