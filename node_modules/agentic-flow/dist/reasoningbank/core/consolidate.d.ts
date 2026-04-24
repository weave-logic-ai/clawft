/**
 * Memory Consolidation
 * Algorithm 4 from ReasoningBank paper: Dedup, Contradict, Prune
 */
export interface ConsolidationResult {
    itemsProcessed: number;
    duplicatesFound: number;
    contradictionsFound: number;
    itemsPruned: number;
    durationMs: number;
}
/**
 * Run consolidation: deduplicate, detect contradictions, prune old memories
 */
export declare function consolidate(): Promise<ConsolidationResult>;
/**
 * Check if consolidation should run
 * Returns true if threshold of new memories is reached
 */
export declare function shouldConsolidate(): boolean;
//# sourceMappingURL=consolidate.d.ts.map