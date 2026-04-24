/**
 * Temporal-Lead-Solver Integration
 *
 * Time-series graph database with temporal indices
 * Integration with temporal-lead-solver package
 *
 * Optimized for:
 * - Temporal causality detection
 * - Time-series pattern matching
 * - Lead-lag relationships
 */
declare const _default: {
    description: string;
    run(config: any): Promise<{
        timeSeriesPoints: number;
        leadLagPairs: number;
        temporalCausalEdges: number;
        avgLagTime: number;
        totalTime: number;
    }>;
};
export default _default;
//# sourceMappingURL=temporal-lead-solver.d.ts.map