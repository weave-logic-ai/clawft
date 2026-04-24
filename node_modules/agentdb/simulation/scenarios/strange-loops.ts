/**
 * Strange Loops Simulation
 *
 * Tests self-referential learning patterns using strange-loops concepts
 * Agents observe their own performance and adapt based on meta-cognitive feedback
 */

import { createUnifiedDatabase } from '../../src/db-unified.js';
import { ReflexionMemory } from '../../src/controllers/ReflexionMemory.js';
import { CausalMemoryGraph } from '../../src/controllers/CausalMemoryGraph.js';
import { EmbeddingService } from '../../src/controllers/EmbeddingService.js';
import * as path from 'path';

export default {
  description: 'Self-referential learning with strange loops and meta-cognition',

  async run(config: any) {
    const { verbosity = 2, depth = 3 } = config;

    if (verbosity >= 2) {
      console.log(`   🔄 Initializing Strange Loops Simulation (depth=${depth})`);
    }

    // Initialize AgentDB
    const embedder = new EmbeddingService({
      model: 'Xenova/all-MiniLM-L6-v2',
      dimension: 384,
      provider: 'transformers'
    });
    await embedder.initialize();

    const db = await createUnifiedDatabase(
      path.join(process.cwd(), 'simulation', 'data', 'strange-loops.graph'),
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
      loops: 0,
      metaLearnings: 0,
      selfReferences: 0,
      adaptations: 0,
      totalTime: 0
    };

    const startTime = performance.now();

    // Level 0: Base action
    const baseActionId = await reflexion.storeEpisode({
      sessionId: 'strange-loop',
      task: 'perform base action',
      reward: 0.70,
      success: true,
      input: 'Initial task',
      output: 'Initial result',
      critique: 'Can be improved'
    });

    results.loops++;

    if (verbosity >= 3) {
      console.log(`      🔹 Level 0: Base action (reward: 0.70)`);
    }

    // Strange loop: Each level observes and improves the previous level
    let previousId = baseActionId;
    let previousReward = 0.70;

    for (let level = 1; level <= depth; level++) {
      // Meta-observation: Observe previous level's performance
      const metaObservation = await reflexion.storeEpisode({
        sessionId: 'strange-loop',
        task: `observe level ${level - 1} performance`,
        reward: previousReward + 0.05, // Slight improvement from observation
        success: true,
        input: `Analyzing level ${level - 1}`,
        output: `Identified improvement opportunities`,
        critique: `Level ${level - 1} critique: reward ${previousReward.toFixed(2)}`
      });

      results.metaLearnings++;

      // Self-reference: Create causal link back to previous level
      await causal.addCausalEdge({
        fromMemoryId: previousId,
        fromMemoryType: 'episode',
        toMemoryId: metaObservation,
        toMemoryType: 'episode',
        similarity: 0.90,
        uplift: 0.05,
        confidence: 0.85,
        sampleSize: 100,
        mechanism: `Meta-observation of level ${level - 1}`
      });

      results.selfReferences++;

      // Adaptation: Apply learnings to create improved action
      const improvedReward = Math.min(0.95, previousReward + 0.08);
      const improvedActionId = await reflexion.storeEpisode({
        sessionId: 'strange-loop',
        task: `perform improved action at level ${level}`,
        reward: improvedReward,
        success: true,
        input: `Enhanced task based on meta-observation`,
        output: `Improved result`,
        critique: `Applied learnings from level ${level - 1}`
      });

      results.adaptations++;

      // Create causal link from meta-observation to improved action
      await causal.addCausalEdge({
        fromMemoryId: metaObservation,
        fromMemoryType: 'episode',
        toMemoryId: improvedActionId,
        toMemoryType: 'episode',
        similarity: 0.95,
        uplift: 0.08,
        confidence: 0.90,
        sampleSize: 100,
        mechanism: `Self-improvement through meta-cognition`
      });

      results.loops++;

      if (verbosity >= 3) {
        console.log(`      🔹 Level ${level}: Meta-observation + Adaptation (reward: ${improvedReward.toFixed(2)})`);
      }

      // The loop: Next iteration observes THIS level's performance
      previousId = improvedActionId;
      previousReward = improvedReward;
    }

    // Note: Causal chain querying requires full SQL migration
    // For now, we track the strange loop structure through episode creation

    if (verbosity >= 2) {
      console.log(`      🔄 Strange Loop Structure: ${baseActionId} → ... → ${previousId}`);
      console.log(`      📊 Loops: ${results.loops}`);
      console.log(`      📊 Meta-learnings: ${results.metaLearnings}`);
      console.log(`      📊 Self-references: ${results.selfReferences}`);
      console.log(`      📊 Adaptations: ${results.adaptations}`);
    }

    if (verbosity >= 3) {
      console.log(`      🔍 Reward Progression: 0.70 → ${previousReward.toFixed(2)} (+${((previousReward - 0.70) * 100).toFixed(1)}%)`);
    }

    const endTime = performance.now();
    results.totalTime = endTime - startTime;

    db.close();

    if (verbosity >= 2) {
      console.log(`      ⏱️  Duration: ${results.totalTime.toFixed(2)}ms`);
    }

    return results;
  }
};
