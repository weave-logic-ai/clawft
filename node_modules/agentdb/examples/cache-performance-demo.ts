/**
 * QueryCache Performance Demonstration
 *
 * This example demonstrates the performance improvements from the LRU query cache
 */

import { AgentDB } from '../src/index.js';

async function runPerformanceDemo() {
  console.log('🚀 AgentDB QueryCache Performance Demo\n');

  // Initialize AgentDB with cache enabled
  const db = new AgentDB({
    dbPath: ':memory:',
    cacheConfig: {
      maxSize: 1000,
      defaultTTL: 300000, // 5 minutes
      enabled: true,
    },
  });

  await db.initialize();

  const memory = db.getController('reflexion');
  const skills = db.getController('skills');

  // Seed test data
  console.log('📝 Seeding test data...');

  const tasks = [
    'implement authentication',
    'create REST API',
    'design database schema',
    'build user interface',
    'write unit tests',
  ];

  // Store episodes
  for (let i = 0; i < 20; i++) {
    const task = tasks[i % tasks.length];
    await memory.storeEpisode({
      sessionId: 'test-session',
      task,
      input: `Attempt ${i + 1} at ${task}`,
      output: `Result of ${task}`,
      critique: i % 2 === 0 ? `Good approach for ${task}` : undefined,
      reward: Math.random(),
      success: Math.random() > 0.3,
      latencyMs: Math.floor(Math.random() * 1000),
      tokensUsed: Math.floor(Math.random() * 500),
    });
  }

  // Store skills
  for (let i = 0; i < 10; i++) {
    const task = tasks[i % tasks.length];
    await skills.createSkill({
      name: `${task}-skill-${i}`,
      description: `Skill for ${task}`,
      signature: {
        inputs: { task: 'string' },
        outputs: { result: 'any' },
      },
      successRate: Math.random(),
      uses: Math.floor(Math.random() * 100),
      avgReward: Math.random(),
      avgLatencyMs: Math.floor(Math.random() * 500),
    });
  }

  console.log('✅ Test data seeded\n');

  // Performance Test 1: ReflexionMemory retrieveRelevant
  console.log('📊 Test 1: ReflexionMemory.retrieveRelevant()');
  console.log('='.repeat(50));

  const testTask = 'implement authentication';

  // First query (cache miss)
  const start1 = performance.now();
  const result1 = await memory.retrieveRelevant({ task: testTask, k: 5 });
  const duration1 = performance.now() - start1;
  console.log(`❌ First query (cache miss):  ${duration1.toFixed(2)}ms`);
  console.log(`   Found ${result1.length} episodes`);

  // Second query (cache hit)
  const start2 = performance.now();
  const result2 = await memory.retrieveRelevant({ task: testTask, k: 5 });
  const duration2 = performance.now() - start2;
  console.log(`✅ Second query (cache hit):  ${duration2.toFixed(2)}ms`);
  console.log(`   Found ${result2.length} episodes`);

  const speedup1 = duration1 / duration2;
  console.log(`⚡ Speedup: ${speedup1.toFixed(1)}x faster\n`);

  // Performance Test 2: SkillLibrary retrieveSkills
  console.log('📊 Test 2: SkillLibrary.retrieveSkills()');
  console.log('='.repeat(50));

  // First query (cache miss)
  const start3 = performance.now();
  const skills1 = await skills.retrieveSkills({ task: testTask, k: 5 });
  const duration3 = performance.now() - start3;
  console.log(`❌ First query (cache miss):  ${duration3.toFixed(2)}ms`);
  console.log(`   Found ${skills1.length} skills`);

  // Second query (cache hit)
  const start4 = performance.now();
  const skills2 = await skills.retrieveSkills({ task: testTask, k: 5 });
  const duration4 = performance.now() - start4;
  console.log(`✅ Second query (cache hit):  ${duration4.toFixed(2)}ms`);
  console.log(`   Found ${skills2.length} skills`);

  const speedup2 = duration3 / duration4;
  console.log(`⚡ Speedup: ${speedup2.toFixed(1)}x faster\n`);

  // Performance Test 3: Multiple repeated queries
  console.log('📊 Test 3: Mixed Workload (10 queries per task)');
  console.log('='.repeat(50));

  const coldStart = performance.now();
  for (const task of tasks) {
    await memory.retrieveRelevant({ task, k: 5 });
  }
  const coldDuration = performance.now() - coldStart;

  // Repeat queries (should hit cache)
  const warmStart = performance.now();
  for (const task of tasks) {
    await memory.retrieveRelevant({ task, k: 5 });
  }
  const warmDuration = performance.now() - warmStart;

  console.log(`❌ Cold queries (5 tasks):    ${coldDuration.toFixed(2)}ms`);
  console.log(`✅ Warm queries (5 tasks):    ${warmDuration.toFixed(2)}ms`);
  const speedup3 = coldDuration / warmDuration;
  console.log(`⚡ Speedup: ${speedup3.toFixed(1)}x faster\n`);

  // Cache Statistics
  console.log('📈 Cache Statistics');
  console.log('='.repeat(50));

  const memoryStats = memory.getCacheStats();
  console.log('ReflexionMemory Cache:');
  console.log(`  Total hits:     ${memoryStats.hits}`);
  console.log(`  Total misses:   ${memoryStats.misses}`);
  console.log(`  Hit rate:       ${memoryStats.hitRate.toFixed(2)}%`);
  console.log(`  Cache size:     ${memoryStats.size}/${memoryStats.capacity}`);
  console.log(`  Evictions:      ${memoryStats.evictions}`);
  console.log(`  Memory used:    ${(memoryStats.memoryUsed / 1024).toFixed(2)} KB`);
  console.log(`  By category:`, memoryStats.entriesByCategory);

  console.log();

  const skillsStats = skills.getCacheStats();
  console.log('SkillLibrary Cache:');
  console.log(`  Total hits:     ${skillsStats.hits}`);
  console.log(`  Total misses:   ${skillsStats.misses}`);
  console.log(`  Hit rate:       ${skillsStats.hitRate.toFixed(2)}%`);
  console.log(`  Cache size:     ${skillsStats.size}/${skillsStats.capacity}`);
  console.log(`  Evictions:      ${skillsStats.evictions}`);
  console.log(`  Memory used:    ${(skillsStats.memoryUsed / 1024).toFixed(2)} KB`);
  console.log(`  By category:`, skillsStats.entriesByCategory);

  // Summary
  console.log('\n' + '='.repeat(50));
  console.log('📊 Performance Summary');
  console.log('='.repeat(50));
  console.log(`Average speedup: ${((speedup1 + speedup2 + speedup3) / 3).toFixed(1)}x`);
  console.log(`Overall hit rate: ${((memoryStats.hitRate + skillsStats.hitRate) / 2).toFixed(2)}%`);
  console.log(
    `Total memory used: ${((memoryStats.memoryUsed + skillsStats.memoryUsed) / 1024).toFixed(2)} KB`
  );

  console.log('\n✅ Demo complete!\n');

  // Cleanup
  await db.close();
}

// Run the demo
if (import.meta.url === `file://${process.argv[1]}`) {
  runPerformanceDemo().catch(console.error);
}

export { runPerformanceDemo };
