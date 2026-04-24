/**
 * Context Synthesis - Generate coherent narratives from multiple memories
 *
 * Takes a collection of retrieved episodes/patterns and synthesizes
 * a coherent context summary with extracted patterns, success rates,
 * and actionable insights.
 */
export interface MemoryPattern {
    task: string;
    reward: number;
    success: boolean;
    critique?: string;
    input?: string;
    output?: string;
    similarity?: number;
    [key: string]: any;
}
export interface SynthesizedContext {
    summary: string;
    patterns: string[];
    successRate: number;
    averageReward: number;
    recommendations: string[];
    keyInsights: string[];
    totalMemories: number;
}
export declare class ContextSynthesizer {
    /**
     * Synthesize context from multiple memories
     *
     * @param memories - Retrieved episodes/patterns
     * @param options - Synthesis options
     * @returns Synthesized context with insights
     */
    static synthesize(memories: MemoryPattern[], options?: {
        minPatternFrequency?: number;
        includeRecommendations?: boolean;
        maxSummaryLength?: number;
    }): SynthesizedContext;
    /**
     * Extract common patterns from memory critiques
     */
    private static extractPatterns;
    /**
     * Extract meaningful phrases from text
     */
    private static extractPhrases;
    /**
     * Generate key insights from memories
     */
    private static generateKeyInsights;
    /**
     * Generate actionable recommendations
     */
    private static generateRecommendations;
    /**
     * Generate narrative summary
     */
    private static generateSummary;
    /**
     * Extract actionable steps from successful memories
     */
    static extractActionableSteps(memories: MemoryPattern[]): string[];
}
//# sourceMappingURL=ContextSynthesizer.d.ts.map