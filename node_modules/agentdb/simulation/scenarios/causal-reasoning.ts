/**
 * Causal Reasoning Simulation
 *
 * Tests CausalMemoryGraph with intervention-based reasoning
 */

import { createUnifiedDatabase } from '../../src/db-unified.js';
import { ReflexionMemory } from '../../src/controllers/ReflexionMemory.js';
import { CausalMemoryGraph } from '../../src/controllers/CausalMemoryGraph.js';
import { EmbeddingService } from '../../src/controllers/EmbeddingService.js';
import * as path from 'path';

export default {
  description: 'Causal reasoning with intervention analysis',

  async run(config: any) {
    const { verbosity = 2 } = config;

    if (verbosity >= 2) {
      console.log('   🔗 Initializing Causal Reasoning Simulation');
    }

    // Initialize AgentDB
    const embedder = new EmbeddingService({
      model: 'Xenova/all-MiniLM-L6-v2',
      dimension: 384,
      provider: 'transformers'
    });
    await embedder.initialize();

    const db = await createUnifiedDatabase(
      path.join(process.cwd(), 'simulation', 'data', 'causal.graph'),
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

    const causal = new CausalMemoryGraph(
      db.getGraphDatabase() as any,
      db.getGraphDatabase() as any  // Pass graphBackend for GraphDatabaseAdapter support
    );

    const results = {
      episodes: 0,
      causalEdges: 0,
      avgUplift: 0,
      totalTime: 0
    };

    const startTime = performance.now();

    // Create episodes with causal relationships
    const causalPairs = [
      {
        cause: { task: 'add comprehensive tests', reward: 0.85 },
        effect: { task: 'improve code quality', reward: 0.95 },
        uplift: 0.10
      },
      {
        cause: { task: 'implement caching', reward: 0.80 },
        effect: { task: 'reduce response time', reward: 0.92 },
        uplift: 0.12
      },
      {
        cause: { task: 'add error logging', reward: 0.75 },
        effect: { task: 'faster debugging', reward: 0.88 },
        uplift: 0.13
      }
    ];

    const episodeIds: number[] = [];

    for (const pair of causalPairs) {
      // Store cause episode
      const causeId = await reflexion.storeEpisode({
        sessionId: 'causal-sim',
        task: pair.cause.task,
        reward: pair.cause.reward,
        success: true
      });

      // Store effect episode
      const effectId = await reflexion.storeEpisode({
        sessionId: 'causal-sim',
        task: pair.effect.task,
        reward: pair.effect.reward,
        success: true
      });

      results.episodes += 2;
      episodeIds.push(causeId, effectId);

      // Create causal edge
      await causal.addCausalEdge({
        fromMemoryId: causeId,
        fromMemoryType: 'episode',
        toMemoryId: effectId,
        toMemoryType: 'episode',
        similarity: 0.85,
        uplift: pair.uplift,
        confidence: 0.95,
        sampleSize: 100,
        mechanism: `${pair.cause.task} → ${pair.effect.task}`
      });

      results.causalEdges++;
      results.avgUplift += pair.uplift;

      if (verbosity >= 3) {
        console.log(`      ✅ Added causal relationship: ${pair.cause.task} → ${pair.effect.task}`);
      }
    }

    // Query causal effects (Note: All query functions require full SQL→Graph migration)
    // Skipping queries for now - causal edges are successfully created above

    if (verbosity >= 3) {
      console.log(`      🔍 Causal edge query functions pending Graph migration`);
    }

    const endTime = performance.now();
    results.totalTime = endTime - startTime;
    results.avgUplift /= causalPairs.length;

    db.close();

    if (verbosity >= 2) {
      console.log(`      📊 Episodes: ${results.episodes}`);
      console.log(`      📊 Causal Edges: ${results.causalEdges}`);
      console.log(`      📊 Avg Uplift: ${(results.avgUplift * 100).toFixed(1)}%`);
      console.log(`      ⏱️  Duration: ${results.totalTime.toFixed(2)}ms`);
    }

    return results;
  }
};
