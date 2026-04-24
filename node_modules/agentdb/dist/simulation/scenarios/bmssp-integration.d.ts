/**
 * BMSSP Integration Simulation
 *
 * Biologically-Motivated Symbolic-Subsymbolic Processing
 * Integration with @ruvnet/bmssp package
 *
 * Dedicated graph DB optimized for symbolic reasoning with:
 * - Symbolic rule graphs
 * - Subsymbolic pattern embeddings
 * - Hybrid reasoning paths
 */
declare const _default: {
    description: string;
    run(config: any): Promise<{
        symbolicRules: number;
        subsymbolicPatterns: number;
        hybridInferences: number;
        avgConfidence: number;
        totalTime: number;
    }>;
};
export default _default;
//# sourceMappingURL=bmssp-integration.d.ts.map