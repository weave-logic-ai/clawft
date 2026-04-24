/**
 * Reflexion Learning Simulation
 *
 * Tests ReflexionMemory with multi-agent learning and self-improvement
 */

import { createUnifiedDatabase } from '../../src/db-unified.js';
import { ReflexionMemory } from '../../src/controllers/ReflexionMemory.js';
import { EmbeddingService } from '../../src/controllers/EmbeddingService.js';
import { PerformanceOptimizer } from '../utils/PerformanceOptimizer.js';
import * as path from 'path';

export default {
  description: 'Multi-agent reflexion learning with episodic memory',

  async run(config: any) {
    const { verbosity = 2 } = config;

    if (verbosity >= 2) {
      console.log('   🧠 Initializing Reflexion Learning Simulation');
    }

    // Initialize performance optimizer
    const optimizer = new PerformanceOptimizer({ batchSize: 20 });

    // Initialize AgentDB
    const embedder = new EmbeddingService({
      model: 'Xenova/all-MiniLM-L6-v2',
      dimension: 384,
      provider: 'transformers'
    });
    await embedder.initialize();

    const db = await createUnifiedDatabase(
      path.join(process.cwd(), 'simulation', 'data', 'reflexion.graph'),
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

    // Simulate learning episodes
    const tasks = [
      { task: 'implement authentication', success: true, reward: 0.95 },
      { task: 'optimize database queries', success: true, reward: 0.88 },
      { task: 'add error handling', success: true, reward: 0.92 },
      { task: 'refactor api endpoints', success: false, reward: 0.45 },
      { task: 'improve test coverage', success: true, reward: 0.90 }
    ];

    const results = {
      stored: 0,
      retrieved: 0,
      avgSimilarity: 0,
      totalTime: 0
    };

    const startTime = performance.now();

    // Store episodes - OPTIMIZED: Batch operations
    for (let i = 0; i < tasks.length; i++) {
      const task = tasks[i];

      optimizer.queueOperation(async () => {
        await reflexion.storeEpisode({
          sessionId: `sim-session-${Math.floor(i / 2)}`,
          task: task.task,
          reward: task.reward,
          success: task.success,
          input: `Task: ${task.task}`,
          output: task.success ? 'Successfully completed' : 'Failed with errors',
          critique: task.success
            ? 'Good implementation, could be improved'
            : 'Need to review error cases'
        });

        results.stored++;

        if (verbosity >= 3) {
          console.log(`      ✅ Stored episode: ${task.task}`);
        }
      });
    }

    // Execute batch operation
    await optimizer.executeBatch();

    // Retrieve similar episodes
    for (const task of tasks) {
      const similar = await reflexion.retrieveRelevant({
        task: task.task,
        k: 3,
        minReward: 0.7
      });

      results.retrieved += similar.length;

      if (similar.length > 0 && similar[0].similarity) {
        results.avgSimilarity += similar[0].similarity;
      }

      if (verbosity >= 3) {
        console.log(`      🔍 Retrieved ${similar.length} similar episodes for: ${task.task}`);
      }
    }

    const endTime = performance.now();
    results.totalTime = endTime - startTime;
    results.avgSimilarity /= tasks.length;

    db.close();

    // Get optimization metrics
    const optimizerMetrics = optimizer.getMetrics();

    if (verbosity >= 2) {
      console.log(`      📊 Stored: ${results.stored} episodes`);
      console.log(`      📊 Retrieved: ${results.retrieved} similar episodes`);
      console.log(`      📊 Avg Similarity: ${results.avgSimilarity.toFixed(3)}`);
      console.log(`      ⏱️  Duration: ${results.totalTime.toFixed(2)}ms`);
      console.log(`      ⚡ Optimization: ${optimizerMetrics.batchOperations} batches, ${optimizerMetrics.avgLatency} avg`);
    }

    return results;
  }
};
