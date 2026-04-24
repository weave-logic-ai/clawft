/**
 * Causal Reasoning Simulation
 *
 * Tests CausalMemoryGraph with intervention-based reasoning
 */
declare const _default: {
    description: string;
    run(config: any): Promise<{
        episodes: number;
        causalEdges: number;
        avgUplift: number;
        totalTime: number;
    }>;
};
export default _default;
//# sourceMappingURL=causal-reasoning.d.ts.map