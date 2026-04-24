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

import { createUnifiedDatabase } from '../../src/db-unified.js';
import { ReflexionMemory } from '../../src/controllers/ReflexionMemory.js';
import { EmbeddingService } from '../../src/controllers/EmbeddingService.js';
import { PerformanceOptimizer, executeParallel } from '../utils/PerformanceOptimizer.js';
import * as path from 'path';

interface Voter {
  id: string;
  ideologyVector: number[]; // 5D political space: [economic, social, environmental, foreign, governance]
  voteHistory: string[];
}

interface Candidate {
  id: string;
  platform: number[]; // Same 5D space
  endorsements: string[];
}

interface VotingRound {
  roundId: number;
  candidates: Candidate[];
  voters: Voter[];
  results: Map<string, number>;
  winner: string;
  consensusScore: number;
}

export default {
  description: 'Democratic voting system with ranked-choice, coalition formation, and consensus emergence',

  async run(config: any) {
    const { verbosity = 2, rounds = 5, voterCount = 50, candidateCount = 7 } = config;

    if (verbosity >= 2) {
      console.log(`   🗳️  Initializing Voting System: ${voterCount} voters, ${candidateCount} candidates, ${rounds} rounds`);
    }

    // Initialize performance optimizer
    const optimizer = new PerformanceOptimizer({ batchSize: 50 });

    const embedder = new EmbeddingService({
      model: 'Xenova/all-MiniLM-L6-v2',
      dimension: 384,
      provider: 'transformers'
    });
    await embedder.initialize();

    const db = await createUnifiedDatabase(
      path.join(process.cwd(), 'simulation', 'data', 'voting-consensus.graph'),
      embedder,
      { forceMode: 'graph' }
    );

    const reflexion = new ReflexionMemory(
      db.getGraphDatabase() as any,
      embedder,
      undefined,
      undefined,
      db.getGraphDatabase() as any
    );

    const results = {
      rounds: 0,
      totalVotes: 0,
      coalitionsFormed: 0,
      consensusEvolution: [] as number[],
      strategicVotingDetected: 0,
      avgPreferenceShift: 0,
      totalTime: 0
    };

    const startTime = performance.now();

    // Initialize voters with random ideologies
    const voters: Voter[] = Array.from({ length: voterCount }, (_, i) => ({
      id: `voter-${i}`,
      ideologyVector: Array.from({ length: 5 }, () => Math.random() * 2 - 1), // -1 to 1
      voteHistory: []
    }));

    // Run voting rounds
    for (let round = 0; round < rounds; round++) {
      // Generate candidates with platforms
      const candidates: Candidate[] = Array.from({ length: candidateCount }, (_, i) => ({
        id: `candidate-R${round}-${i}`,
        platform: Array.from({ length: 5 }, () => Math.random() * 2 - 1),
        endorsements: []
      }));

      // Each voter ranks candidates by preference (euclidean distance in ideology space)
      const ballots = new Map<string, string[]>();

      for (const voter of voters) {
        // Calculate distance to each candidate
        const preferences = candidates.map(candidate => {
          const distance = Math.sqrt(
            voter.ideologyVector.reduce((sum, val, idx) =>
              sum + Math.pow(val - candidate.platform[idx], 2),
              0
            )
          );
          return { candidateId: candidate.id, distance };
        });

        // Sort by closest (lowest distance) = highest preference
        preferences.sort((a, b) => a.distance - b.distance);
        ballots.set(voter.id, preferences.map(p => p.candidateId));
      }

      // Ranked-Choice Voting algorithm
      const eliminated = new Set<string>();
      let winner: string | null = null;
      let voteCounts = new Map<string, number>();

      while (!winner && eliminated.size < candidates.length - 1) {
        voteCounts.clear();

        // Count first-choice votes (excluding eliminated)
        for (const [voterId, ranked] of ballots.entries()) {
          const firstChoice = ranked.find(c => !eliminated.has(c));
          if (firstChoice) {
            voteCounts.set(firstChoice, (voteCounts.get(firstChoice) || 0) + 1);
          }
        }

        // Check for majority winner (>50%)
        const majority = voterCount / 2;
        for (const [candidateId, count] of voteCounts.entries()) {
          if (count > majority) {
            winner = candidateId;
            break;
          }
        }

        if (!winner) {
          // Eliminate candidate with fewest votes
          let minVotes = Infinity;
          let toEliminate = '';
          for (const [candidateId, count] of voteCounts.entries()) {
            if (count < minVotes) {
              minVotes = count;
              toEliminate = candidateId;
            }
          }
          eliminated.add(toEliminate);
        }
      }

      if (!winner) {
        winner = candidates.find(c => !eliminated.has(c.id))!.id;
      }

      // Calculate consensus score (how concentrated the final vote was)
      const winnerVotes = voteCounts.get(winner!) || 0;
      const consensusScore = winnerVotes / voterCount;

      results.consensusEvolution.push(consensusScore);

      // Voters learn from outcomes - OPTIMIZED: Batch database operations
      const winningCandidate = candidates.find(c => c.id === winner)!;

      // Queue all episode storage operations
      for (let i = 0; i < Math.min(10, voters.length); i++) {
        const voter = voters[i];

        optimizer.queueOperation(async () => {
          return reflexion.storeEpisode({
            sessionId: `round-${round}`,
            task: `vote in election round ${round}`,
            input: `Voter ideology: ${voter.ideologyVector.join(',')}`,
            output: `Voted for: ${ballots.get(voter.id)?.[0]}, Winner: ${winner}`,
            reward: consensusScore,
            success: ballots.get(voter.id)?.[0] === winner,
            critique: `Consensus: ${(consensusScore * 100).toFixed(1)}%`,
            metadata: {
              voterIdeology: voter.ideologyVector,
              winnerPlatform: winningCandidate.platform,
              roundNumber: round
            }
          });
        });
      }

      // Execute batch operation
      await optimizer.executeBatch();

      // Detect coalitions (clusters of voters with similar preferences)
      const coalitionThreshold = 0.3;
      let coalitions = 0;
      for (let i = 0; i < voters.length; i++) {
        for (let j = i + 1; j < voters.length; j++) {
          const distance = Math.sqrt(
            voters[i].ideologyVector.reduce((sum, val, idx) =>
              sum + Math.pow(val - voters[j].ideologyVector[idx], 2),
              0
            )
          );
          if (distance < coalitionThreshold) {
            coalitions++;
          }
        }
      }
      results.coalitionsFormed += coalitions;

      results.rounds++;
      results.totalVotes += voterCount;

      if (verbosity >= 3) {
        console.log(`      🗳️  Round ${round + 1}: Winner ${winner}, Consensus ${(consensusScore * 100).toFixed(1)}%, Coalitions: ${coalitions}`);
      }
    }

    const endTime = performance.now();
    results.totalTime = endTime - startTime;

    // Analyze consensus evolution
    const initialConsensus = results.consensusEvolution[0];
    const finalConsensus = results.consensusEvolution[results.consensusEvolution.length - 1];
    results.avgPreferenceShift = finalConsensus - initialConsensus;

    db.close();

    // Get optimization metrics
    const optimizerMetrics = optimizer.getMetrics();

    if (verbosity >= 2) {
      console.log(`      📊 Rounds: ${results.rounds}`);
      console.log(`      📊 Total Votes: ${results.totalVotes}`);
      console.log(`      📊 Coalitions Formed: ${results.coalitionsFormed}`);
      console.log(`      📊 Consensus Evolution: ${initialConsensus.toFixed(2)} → ${finalConsensus.toFixed(2)} (${results.avgPreferenceShift > 0 ? '+' : ''}${(results.avgPreferenceShift * 100).toFixed(1)}%)`);
      console.log(`      ⏱️  Duration: ${results.totalTime.toFixed(2)}ms`);
      console.log(`      ⚡ Optimization: ${optimizerMetrics.batchOperations} batches, ${optimizerMetrics.avgLatency} avg`);
    }

    return results;
  }
};
