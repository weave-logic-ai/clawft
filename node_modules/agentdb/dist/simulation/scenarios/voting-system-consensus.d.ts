/**
 * Voting System Consensus Simulation
 *
 * Models a multi-agent democratic voting system with:
 * - Ranked-choice voting (RCV) algorithms
 * - Voter preference aggregation
 * - Coalition formation dynamics
 * - Strategic voting detection
 * - Consensus emergence patterns
 *
 * Tests AgentDB's ability to handle complex multi-agent decision-making
 * and preference learning across voting cycles.
 */
declare const _default: {
    description: string;
    run(config: any): Promise<{
        rounds: number;
        totalVotes: number;
        coalitionsFormed: number;
        consensusEvolution: number[];
        strategicVotingDetected: number;
        avgPreferenceShift: number;
        totalTime: number;
    }>;
};
export default _default;
//# sourceMappingURL=voting-system-consensus.d.ts.map