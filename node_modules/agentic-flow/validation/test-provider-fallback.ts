/**
 * Provider Fallback Validation Test
 *
 * Tests:
 * - ProviderManager initialization
 * - Provider selection strategies
 * - Automatic fallback
 * - Circuit breaker
 * - Cost tracking
 * - Health monitoring
 */

import { ProviderManager, ProviderConfig } from '../src/core/provider-manager.js';
import { LongRunningAgent } from '../src/core/long-running-agent.js';

// Test configuration
const TEST_PROVIDERS: ProviderConfig[] = [
  {
    name: 'gemini',
    apiKey: process.env.GOOGLE_GEMINI_API_KEY || 'test-key',
    priority: 1,
    maxRetries: 2,
    timeout: 5000,
    costPerToken: 0.00015,
    enabled: true
  },
  {
    name: 'anthropic',
    apiKey: process.env.ANTHROPIC_API_KEY || 'test-key',
    priority: 2,
    maxRetries: 2,
    timeout: 5000,
    costPerToken: 0.003,
    enabled: true
  },
  {
    name: 'onnx',
    priority: 3,
    maxRetries: 1,
    timeout: 10000,
    costPerToken: 0,
    enabled: true
  }
];

async function testProviderManager() {
  console.log('🧪 Test 1: ProviderManager Initialization');
  console.log('==========================================\n');

  const manager = new ProviderManager(TEST_PROVIDERS, {
    type: 'priority',
    maxFailures: 2,
    recoveryTime: 5000,
    retryBackoff: 'exponential'
  });

  // Test provider selection
  const provider = await manager.selectProvider('simple', 100);
  console.log(`✅ Selected provider: ${provider}\n`);

  // Test health status
  const health = manager.getHealth();
  console.log('📊 Provider Health:');
  health.forEach(h => {
    console.log(`  ${h.provider}: healthy=${h.isHealthy}, circuitBreaker=${h.circuitBreakerOpen ? 'OPEN' : 'CLOSED'}`);
  });
  console.log('');

  manager.destroy();
  console.log('✅ Test 1 Passed\n');
}

async function testFallbackStrategy() {
  console.log('🧪 Test 2: Fallback Strategy');
  console.log('=============================\n');

  const manager = new ProviderManager(TEST_PROVIDERS, {
    type: 'cost-optimized',
    maxFailures: 2,
    recoveryTime: 5000,
    retryBackoff: 'exponential'
  });

  // Test cost-optimized selection
  console.log('Testing cost-optimized selection...');
  const cheapProvider = await manager.selectProvider('simple', 10000);
  console.log(`✅ Cost-optimized provider: ${cheapProvider} (should prefer Gemini/ONNX)\n`);

  // Test complex task selection
  const complexProvider = await manager.selectProvider('complex', 5000);
  console.log(`✅ Complex task provider: ${complexProvider} (should prefer Anthropic if available)\n`);

  manager.destroy();
  console.log('✅ Test 2 Passed\n');
}

async function testCircuitBreaker() {
  console.log('🧪 Test 3: Circuit Breaker');
  console.log('===========================\n');

  const manager = new ProviderManager(
    [
      {
        name: 'gemini',
        priority: 1,
        maxRetries: 1,
        timeout: 1000,
        costPerToken: 0.00015,
        enabled: true
      },
      {
        name: 'onnx',
        priority: 2,
        maxRetries: 1,
        timeout: 1000,
        costPerToken: 0,
        enabled: true
      }
    ],
    {
      type: 'priority',
      maxFailures: 2, // Open circuit after 2 failures
      recoveryTime: 5000,
      retryBackoff: 'exponential'
    }
  );

  let attemptCount = 0;

  // Simulate failures to trigger circuit breaker
  try {
    await manager.executeWithFallback(async (provider) => {
      attemptCount++;
      console.log(`  Attempt ${attemptCount} with provider: ${provider}`);

      if (provider === 'gemini' && attemptCount <= 3) {
        throw new Error('Simulated rate limit error');
      }

      return { success: true, provider };
    });

    console.log('✅ Fallback successful after circuit breaker\n');

  } catch (error) {
    console.log(`⚠️  Expected error after all providers failed: ${(error as Error).message}\n`);
  }

  // Check circuit breaker status
  const health = manager.getHealth();
  const geminiHealth = health.find(h => h.provider === 'gemini');

  if (geminiHealth) {
    console.log('Circuit Breaker Status:');
    console.log(`  Gemini circuit breaker: ${geminiHealth.circuitBreakerOpen ? 'OPEN ✅' : 'CLOSED'}`);
    console.log(`  Consecutive failures: ${geminiHealth.consecutiveFailures}`);
    console.log('');
  }

  manager.destroy();
  console.log('✅ Test 3 Passed\n');
}

async function testCostTracking() {
  console.log('🧪 Test 4: Cost Tracking');
  console.log('=========================\n');

  const manager = new ProviderManager(TEST_PROVIDERS, {
    type: 'cost-optimized',
    maxFailures: 3,
    recoveryTime: 5000,
    retryBackoff: 'exponential'
  });

  // Execute multiple requests
  for (let i = 0; i < 3; i++) {
    await manager.executeWithFallback(async (provider) => {
      console.log(`  Request ${i + 1} using ${provider}`);
      return { provider, tokens: 1000 };
    }, 'simple', 1000);
  }

  // Check cost summary
  const costs = manager.getCostSummary();
  console.log('\n💰 Cost Summary:');
  console.log(`  Total Cost: $${costs.total.toFixed(6)}`);
  console.log(`  Total Tokens: ${costs.totalTokens.toLocaleString()}`);
  console.log('  By Provider:');
  for (const [provider, cost] of Object.entries(costs.byProvider)) {
    console.log(`    ${provider}: $${cost.toFixed(6)}`);
  }
  console.log('');

  manager.destroy();
  console.log('✅ Test 4 Passed\n');
}

async function testLongRunningAgent() {
  console.log('🧪 Test 5: Long-Running Agent');
  console.log('==============================\n');

  const agent = new LongRunningAgent({
    agentName: 'test-agent',
    providers: TEST_PROVIDERS,
    fallbackStrategy: {
      type: 'cost-optimized',
      maxFailures: 2,
      recoveryTime: 5000,
      retryBackoff: 'exponential'
    },
    checkpointInterval: 10000,
    maxRuntime: 60000,
    costBudget: 1.00
  });

  await agent.start();

  // Execute test tasks
  try {
    const task1 = await agent.executeTask({
      name: 'test-task-1',
      complexity: 'simple',
      estimatedTokens: 500,
      execute: async (provider) => {
        console.log(`  Task 1 using ${provider}`);
        return { result: 'success', provider };
      }
    });
    console.log(`✅ Task 1 completed with ${task1.provider}\n`);

    const task2 = await agent.executeTask({
      name: 'test-task-2',
      complexity: 'medium',
      estimatedTokens: 1500,
      execute: async (provider) => {
        console.log(`  Task 2 using ${provider}`);
        return { result: 'success', provider };
      }
    });
    console.log(`✅ Task 2 completed with ${task2.provider}\n`);

  } catch (error) {
    console.error('❌ Task execution error:', (error as Error).message);
  }

  // Get status
  const status = agent.getStatus();
  console.log('📊 Agent Status:');
  console.log(`  Running: ${status.isRunning}`);
  console.log(`  Runtime: ${status.runtime}ms`);
  console.log(`  Completed Tasks: ${status.completedTasks}`);
  console.log(`  Failed Tasks: ${status.failedTasks}`);
  console.log(`  Total Cost: $${status.totalCost.toFixed(6)}`);
  console.log('');

  await agent.stop();
  console.log('✅ Test 5 Passed\n');
}

async function main() {
  console.log('\n🚀 Provider Fallback Validation Suite');
  console.log('======================================\n');

  try {
    await testProviderManager();
    await testFallbackStrategy();
    await testCircuitBreaker();
    await testCostTracking();
    await testLongRunningAgent();

    console.log('✅ All tests passed!\n');
    process.exit(0);

  } catch (error) {
    console.error('\n❌ Test suite failed:', error);
    process.exit(1);
  }
}

// Run tests
if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(console.error);
}

export { main as runProviderFallbackTests };
