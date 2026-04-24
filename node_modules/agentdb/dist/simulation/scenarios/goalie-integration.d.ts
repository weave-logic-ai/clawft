/**
 * Goalie Integration (Goal-Oriented AI Learning Engine)
 *
 * Goal-tracking graph DB with achievement trees
 * Integration with goalie package
 *
 * Features:
 * - Hierarchical goal decomposition
 * - Subgoal dependency tracking
 * - Achievement progress monitoring
 * - Adaptive goal prioritization
 */
declare const _default: {
    description: string;
    run(config: any): Promise<{
        primaryGoals: number;
        subgoals: number;
        achievements: number;
        avgProgress: number;
        totalTime: number;
    }>;
};
export default _default;
//# sourceMappingURL=goalie-integration.d.ts.map