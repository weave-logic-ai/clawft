/**
 * Federated Learning Example with SONA v0.1.4
 *
 * Demonstrates distributed learning across multiple agents with central coordination
 */

import { EphemeralLearningAgent, FederatedLearningCoordinator, FederatedLearningManager } from '../src/services/federated-learning';

/**
 * Example 1: Basic Ephemeral Agent
 */
async function example1_ephemeralAgent() {
  console.log('\n=== Example 1: Ephemeral Learning Agent ===\n');

  // Create agent (mock SONA engine for now)
  const agent = new EphemeralLearningAgent({
    agentId: 'agent-1',
    minQuality: 0.7,
    qualityFiltering: true
  });

  // Initialize with SONA engine
  await agent.initialize({} as any); // Would use real SONA engine

  // Process tasks
  console.log('Processing tasks...');
  await agent.processTask(new Float32Array([0.1, 0.2, 0.3]), 0.85);
  await agent.processTask(new Float32Array([0.2, 0.3, 0.4]), 0.90);
  await agent.processTask(new Float32Array([0.3, 0.4, 0.5]), 0.65); // Below threshold

  console.log(`Tasks processed: ${agent.getTaskCount()}`);

  // Export state for federation
  const state = agent.exportState();
  console.log('\nExported State:');
  console.log('  Agent ID:', state.agentId);
  console.log('  Quality:', state.quality.toFixed(3));
  console.log('  Embedding:', Array.from(state.embedding).map(v => v.toFixed(3)).join(', '));
  console.log('  Metadata:', state.metadata);
}

/**
 * Example 2: Federated Coordinator
 */
async function example2_coordinator() {
  console.log('\n=== Example 2: Federated Coordinator ===\n');

  // Create coordinator
  const coordinator = new FederatedLearningCoordinator({
    agentId: 'coordinator-1',
    minQuality: 0.7,
    qualityFiltering: true,
    maxAgents: 10
  });

  // Simulate receiving states from multiple agents
  console.log('Aggregating agent states...');

  await coordinator.aggregate({
    agentId: 'agent-1',
    embedding: new Float32Array([0.1, 0.2, 0.3]),
    quality: 0.85,
    timestamp: Date.now()
  });

  await coordinator.aggregate({
    agentId: 'agent-2',
    embedding: new Float32Array([0.2, 0.3, 0.4]),
    quality: 0.90,
    timestamp: Date.now()
  });

  await coordinator.aggregate({
    agentId: 'agent-3',
    embedding: new Float32Array([0.15, 0.25, 0.35]),
    quality: 0.80,
    timestamp: Date.now()
  });

  console.log(`\nAggregated ${coordinator.getAgentCount()} agents`);

  // Consolidate states
  console.log('\nConsolidating...');
  const consolidated = await coordinator.consolidate();

  console.log('\nConsolidated State:');
  console.log('  Quality:', consolidated.quality.toFixed(3));
  console.log('  Embedding:', Array.from(consolidated.embedding).map(v => v.toFixed(3)).join(', '));
  console.log('  Metadata:', consolidated.metadata);

  // Get summary
  const summary = coordinator.getSummary();
  console.log('\nCoordinator Summary:');
  console.log('  Agents:', summary.agentCount);
  console.log('  Avg Quality:', summary.avgQuality.toFixed(3));
  console.log('  Quality Range:', `${summary.minQuality.toFixed(3)} - ${summary.maxQuality.toFixed(3)}`);
}

/**
 * Example 3: Federated Learning Manager (Full System)
 */
async function example3_fullSystem() {
  console.log('\n=== Example 3: Complete Federated Learning System ===\n');

  // Create manager
  const manager = new FederatedLearningManager({
    agentId: 'coordinator',
    minQuality: 0.7,
    aggregationInterval: 5000 // 5 seconds
  });

  // Register multiple agents
  console.log('Registering agents...');
  const agent1 = manager.registerAgent('agent-1', {} as any);
  const agent2 = manager.registerAgent('agent-2', {} as any);
  const agent3 = manager.registerAgent('agent-3', {} as any);

  console.log('Registered 3 agents\n');

  // Simulate distributed learning
  console.log('Agents processing tasks...');

  // Agent 1 tasks
  await agent1.processTask(new Float32Array([0.1, 0.2, 0.3]), 0.85);
  await agent1.processTask(new Float32Array([0.15, 0.25, 0.35]), 0.88);

  // Agent 2 tasks
  await agent2.processTask(new Float32Array([0.2, 0.3, 0.4]), 0.90);
  await agent2.processTask(new Float32Array([0.25, 0.35, 0.45]), 0.92);

  // Agent 3 tasks
  await agent3.processTask(new Float32Array([0.12, 0.22, 0.32]), 0.80);
  await agent3.processTask(new Float32Array([0.18, 0.28, 0.38]), 0.83);

  console.log('Tasks processed by all agents\n');

  // Get initial summary
  let summary = manager.getSummary();
  console.log('System Summary (before aggregation):');
  console.log('  Total Agents:', summary.agents.count);
  console.log('  Active Agents:', summary.agents.activeAgents.length);
  console.log('  Coordinator Agents:', summary.coordinator.agentCount);

  // Manually trigger aggregation
  console.log('\nTriggering aggregation...');
  await manager.aggregateAll();

  // Get updated summary
  summary = manager.getSummary();
  console.log('\nSystem Summary (after aggregation):');
  console.log('  Total Agents:', summary.agents.count);
  console.log('  Active Agents:', summary.agents.activeAgents.length);
  console.log('  Coordinator:');
  console.log('    - Aggregated Agents:', summary.coordinator.agentCount);
  console.log('    - Avg Quality:', summary.coordinator.avgQuality.toFixed(3));
  console.log('    - Consolidated:', summary.coordinator.consolidated);

  // Cleanup
  manager.cleanup();
  console.log('\nSystem cleaned up');
}

/**
 * Example 4: Quality Filtering
 */
async function example4_qualityFiltering() {
  console.log('\n=== Example 4: Quality-Based Filtering ===\n');

  const coordinator = new FederatedLearningCoordinator({
    agentId: 'coordinator',
    minQuality: 0.8, // High quality threshold
    qualityFiltering: true
  });

  console.log('Submitting agent states (min quality: 0.8)...\n');

  // High quality - should be accepted
  await coordinator.aggregate({
    agentId: 'agent-high-1',
    embedding: new Float32Array([0.1, 0.2, 0.3]),
    quality: 0.95,
    timestamp: Date.now()
  });
  console.log('✅ agent-high-1 (quality: 0.95) accepted');

  // Medium quality - should be rejected
  await coordinator.aggregate({
    agentId: 'agent-medium-1',
    embedding: new Float32Array([0.2, 0.3, 0.4]),
    quality: 0.75,
    timestamp: Date.now()
  });
  console.log('❌ agent-medium-1 (quality: 0.75) rejected');

  // High quality - should be accepted
  await coordinator.aggregate({
    agentId: 'agent-high-2',
    embedding: new Float32Array([0.15, 0.25, 0.35]),
    quality: 0.88,
    timestamp: Date.now()
  });
  console.log('✅ agent-high-2 (quality: 0.88) accepted');

  const summary = coordinator.getSummary();
  console.log(`\nFinal: ${summary.agentCount} high-quality agents aggregated`);
  console.log(`Average quality: ${summary.avgQuality.toFixed(3)}`);
}

/**
 * Example 5: Large-Scale Federation (100+ agents)
 */
async function example5_largescale() {
  console.log('\n=== Example 5: Large-Scale Federation ===\n');

  const coordinator = new FederatedLearningCoordinator({
    agentId: 'coordinator',
    minQuality: 0.7,
    maxAgents: 50 // Limit to 50 most recent agents
  });

  console.log('Simulating 100 distributed agents...');

  // Simulate 100 agents reporting
  for (let i = 0; i < 100; i++) {
    await coordinator.aggregate({
      agentId: `agent-${i}`,
      embedding: new Float32Array([
        Math.random(),
        Math.random(),
        Math.random()
      ]),
      quality: 0.7 + Math.random() * 0.3, // Quality between 0.7-1.0
      timestamp: Date.now() + i // Stagger timestamps
    });

    if ((i + 1) % 20 === 0) {
      process.stdout.write(`  ${i + 1} agents processed...\n`);
    }
  }

  console.log('\nConsolidating...');
  const consolidated = await coordinator.consolidate();

  const summary = coordinator.getSummary();
  console.log('\nLarge-Scale Summary:');
  console.log('  Agents Aggregated:', summary.agentCount, '(limited to 50)');
  console.log('  Avg Quality:', summary.avgQuality.toFixed(3));
  console.log('  Quality Range:', `${summary.minQuality.toFixed(3)} - ${summary.maxQuality.toFixed(3)}`);
  console.log('  Consolidated Quality:', consolidated.quality.toFixed(3));
}

/**
 * Run all examples
 */
async function main() {
  console.log('╔══════════════════════════════════════════════════╗');
  console.log('║   Federated Learning Examples - SONA v0.1.4    ║');
  console.log('╚══════════════════════════════════════════════════╝');

  try {
    await example1_ephemeralAgent();
    await example2_coordinator();
    await example3_fullSystem();
    await example4_qualityFiltering();
    await example5_largescale();

    console.log('\n✅ All examples completed successfully!');
  } catch (error) {
    console.error('\n❌ Example failed:', error);
    process.exit(1);
  }
}

// Run if executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}

export {
  example1_ephemeralAgent,
  example2_coordinator,
  example3_fullSystem,
  example4_qualityFiltering,
  example5_largescale
};
