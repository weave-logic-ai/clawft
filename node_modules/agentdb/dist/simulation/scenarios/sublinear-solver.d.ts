/**
 * Sublinear-Time-Solver Integration
 *
 * O(log n) query optimization with sublinear-time-solver package
 *
 * Dedicated vector DB optimized for:
 * - Logarithmic search complexity
 * - HNSW indexing
 * - Approximate nearest neighbor (ANN) queries
 */
declare const _default: {
    description: string;
    run(config: any): Promise<{
        insertions: number;
        queries: number;
        avgQueryTime: number;
        complexity: string;
        totalTime: number;
    }>;
};
export default _default;
//# sourceMappingURL=sublinear-solver.d.ts.map