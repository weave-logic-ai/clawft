/**
 * Comprehensive E2E Test for ReasoningBank WASM Integration
 *
 * Tests all functionality in a clean Docker environment simulating remote npx usage.
 * Validates: WASM loading, storage operations, API completeness, performance, memory.
 */

import { ReasoningBankAdapter, createReasoningBank, type PatternInput } from '../dist/reasoningbank/wasm-adapter.js';
import { existsSync, statSync } from 'fs';
import { resolve } from 'path';

interface TestResult {
  name: string;
  passed: boolean;
  duration: number;
  error?: string;
  details?: any;
}

const results: TestResult[] = [];

function test(name: string, fn: () => Promise<void>) {
  return async (): Promise<TestResult> => {
    const start = Date.now();
    try {
      await fn();
      const duration = Date.now() - start;
      console.log(`✅ ${name} (${duration}ms)`);
      return { name, passed: true, duration };
    } catch (error) {
      const duration = Date.now() - start;
      const errorMessage = error instanceof Error ? error.message : String(error);
      console.error(`❌ ${name} (${duration}ms):`, errorMessage);
      return { name, passed: false, duration, error: errorMessage };
    }
  };
}

// Test Suite
const tests = {
  'WASM Files Exist': test('WASM Files Exist', async () => {
    const wasmPath = resolve(import.meta.dirname, '../wasm/reasoningbank/reasoningbank_wasm_bg.wasm');
    if (!existsSync(wasmPath)) {
      throw new Error(`WASM file not found at ${wasmPath}`);
    }
    const stats = statSync(wasmPath);
    if (stats.size < 100000) {
      throw new Error(`WASM file too small (${stats.size} bytes)`);
    }
    console.log(`   WASM size: ${(stats.size / 1024).toFixed(1)}KB`);
  }),

  'TypeScript Wrapper Compiled': test('TypeScript Wrapper Compiled', async () => {
    const wrapperPath = resolve(import.meta.dirname, '../dist/reasoningbank/wasm-adapter.js');
    if (!existsSync(wrapperPath)) {
      throw new Error(`TypeScript wrapper not found at ${wrapperPath}`);
    }
  }),

  'Create ReasoningBank Instance': test('Create ReasoningBank Instance', async () => {
    const rb = new ReasoningBankAdapter('e2e-test-instance');
    const stats = await rb.getStats();
    if (typeof stats.total_patterns !== 'number') {
      throw new Error('Invalid stats structure');
    }
  }),

  'Store Pattern': test('Store Pattern', async () => {
    const rb = await createReasoningBank('e2e-test-store');
    const pattern: PatternInput = {
      task_description: 'E2E test pattern for WASM validation',
      task_category: 'e2e-testing',
      strategy: 'comprehensive-validation',
      success_score: 0.98,
      duration_seconds: 2.5,
    };
    const id = await rb.storePattern(pattern);
    if (!id || typeof id !== 'string') {
      throw new Error('Pattern ID not returned');
    }
    console.log(`   Pattern ID: ${id.substring(0, 8)}...`);
  }),

  'Retrieve Pattern': test('Retrieve Pattern', async () => {
    const rb = await createReasoningBank('e2e-test-retrieve');
    const pattern: PatternInput = {
      task_description: 'Test retrieval pattern',
      task_category: 'retrieval-test',
      strategy: 'test-strategy',
      success_score: 0.95,
    };
    const id = await rb.storePattern(pattern);
    const retrieved = await rb.getPattern(id);
    if (!retrieved) {
      throw new Error('Pattern not retrieved');
    }
    if (retrieved.task_description !== pattern.task_description) {
      throw new Error('Pattern data mismatch');
    }
  }),

  'Search by Category': test('Search by Category', async () => {
    const rb = await createReasoningBank('e2e-test-search');
    // Store multiple patterns
    for (let i = 0; i < 5; i++) {
      await rb.storePattern({
        task_description: `Search test pattern ${i}`,
        task_category: 'search-category',
        strategy: `strategy-${i}`,
        success_score: 0.8 + (i * 0.02),
      });
    }
    const results = await rb.searchByCategory('search-category', 10);
    if (results.length < 5) {
      throw new Error(`Expected at least 5 patterns, got ${results.length}`);
    }
    console.log(`   Found ${results.length} patterns`);
  }),

  'Find Similar Patterns': test('Find Similar Patterns', async () => {
    const rb = await createReasoningBank('e2e-test-similar');
    // Store reference patterns
    await rb.storePattern({
      task_description: 'Optimize database query performance',
      task_category: 'performance',
      strategy: 'indexing',
      success_score: 0.92,
    });
    await rb.storePattern({
      task_description: 'Improve API response time',
      task_category: 'performance',
      strategy: 'caching',
      success_score: 0.88,
    });

    // Search for similar
    const similar = await rb.findSimilar(
      'Speed up database operations',
      'performance',
      5
    );
    if (similar.length === 0) {
      throw new Error('No similar patterns found');
    }
    console.log(`   Found ${similar.length} similar, top score: ${similar[0].similarity_score.toFixed(3)}`);
  }),

  'Get Storage Statistics': test('Get Storage Statistics', async () => {
    const rb = await createReasoningBank('e2e-test-stats');
    const stats = await rb.getStats();
    if (typeof stats.total_patterns !== 'number') {
      throw new Error('Invalid total_patterns');
    }
    if (typeof stats.total_categories !== 'number') {
      throw new Error('Invalid total_categories');
    }
    console.log(`   Stats: ${stats.total_patterns} patterns, ${stats.total_categories} categories`);
  }),

  'Concurrent Operations': test('Concurrent Operations', async () => {
    const rb = await createReasoningBank('e2e-test-concurrent');
    const operations = Array.from({ length: 10 }, (_, i) =>
      rb.storePattern({
        task_description: `Concurrent pattern ${i}`,
        task_category: 'concurrency-test',
        strategy: 'parallel',
        success_score: 0.9,
      })
    );
    const ids = await Promise.all(operations);
    if (ids.length !== 10) {
      throw new Error(`Expected 10 IDs, got ${ids.length}`);
    }
    if (new Set(ids).size !== 10) {
      throw new Error('Duplicate IDs detected');
    }
  }),

  'Performance Benchmark': test('Performance Benchmark', async () => {
    const rb = await createReasoningBank('e2e-test-perf');
    const iterations = 50;
    const start = Date.now();

    for (let i = 0; i < iterations; i++) {
      await rb.storePattern({
        task_description: `Performance test pattern ${i}`,
        task_category: 'benchmark',
        strategy: 'speed-test',
        success_score: 0.85,
      });
    }

    const duration = Date.now() - start;
    const avgTime = duration / iterations;
    console.log(`   ${iterations} ops in ${duration}ms (avg: ${avgTime.toFixed(2)}ms/op)`);

    if (avgTime > 100) {
      throw new Error(`Performance too slow: ${avgTime}ms/op (expected <100ms)`);
    }
  }),

  'Memory Stability': test('Memory Stability', async () => {
    const initialMem = process.memoryUsage().heapUsed;
    const rb = await createReasoningBank('e2e-test-memory');

    // Store 100 patterns
    for (let i = 0; i < 100; i++) {
      await rb.storePattern({
        task_description: `Memory test pattern ${i}`,
        task_category: 'memory-test',
        strategy: 'stability',
        success_score: 0.9,
      });
    }

    // Force garbage collection if available
    if (global.gc) {
      global.gc();
    }

    const finalMem = process.memoryUsage().heapUsed;
    const memDelta = (finalMem - initialMem) / 1024 / 1024;
    console.log(`   Memory delta: ${memDelta.toFixed(2)}MB`);

    if (memDelta > 50) {
      throw new Error(`Memory leak detected: ${memDelta}MB increase`);
    }
  }),

  'Error Handling': test('Error Handling', async () => {
    const rb = await createReasoningBank('e2e-test-errors');

    // Test invalid UUID
    try {
      await rb.getPattern('invalid-uuid');
      throw new Error('Should have thrown error for invalid UUID');
    } catch (error) {
      if (error instanceof Error && error.message.includes('Should have thrown')) {
        throw error;
      }
      // Expected error
    }

    // Test empty category
    const results = await rb.searchByCategory('nonexistent-category', 10);
    if (!Array.isArray(results)) {
      throw new Error('Should return empty array for nonexistent category');
    }
  }),

  'Zero Regression Check': test('Zero Regression Check', async () => {
    // Verify all existing APIs still work
    const rb = await createReasoningBank('e2e-regression-check');

    // Test all methods exist
    const methods = ['storePattern', 'getPattern', 'searchByCategory', 'findSimilar', 'getStats'];
    for (const method of methods) {
      if (typeof (rb as any)[method] !== 'function') {
        throw new Error(`Missing method: ${method}`);
      }
    }

    // Test basic workflow
    const id = await rb.storePattern({
      task_description: 'Regression test',
      task_category: 'regression',
      strategy: 'verification',
      success_score: 1.0,
    });

    const pattern = await rb.getPattern(id);
    if (!pattern) throw new Error('Regression: Pattern not stored');

    const similar = await rb.findSimilar('Regression verification', 'regression', 5);
    if (!Array.isArray(similar)) throw new Error('Regression: findSimilar broken');

    const stats = await rb.getStats();
    if (!stats.total_patterns) throw new Error('Regression: stats broken');
  }),
};

// Run all tests
async function runE2ETests() {
  console.log('\n🧪 Starting Comprehensive E2E Test Suite for WASM Integration\n');
  console.log('=' .repeat(70));
  console.log();

  const startTime = Date.now();

  for (const [name, testFn] of Object.entries(tests)) {
    const result = await testFn();
    results.push(result);
  }

  const totalDuration = Date.now() - startTime;

  console.log();
  console.log('='.repeat(70));
  console.log('\n📊 Test Results Summary\n');

  const passed = results.filter(r => r.passed).length;
  const failed = results.filter(r => r.passed === false).length;

  console.log(`Total Tests: ${results.length}`);
  console.log(`✅ Passed: ${passed}`);
  console.log(`❌ Failed: ${failed}`);
  console.log(`⏱️  Total Duration: ${totalDuration}ms`);
  console.log();

  if (failed > 0) {
    console.log('Failed Tests:');
    results
      .filter(r => !r.passed)
      .forEach(r => console.log(`  - ${r.name}: ${r.error}`));
    console.log();
  }

  // Performance summary
  const perfResult = results.find(r => r.name === 'Performance Benchmark');
  if (perfResult?.passed) {
    console.log('✅ Performance: WASM operations meeting targets');
  }

  // Memory summary
  const memResult = results.find(r => r.name === 'Memory Stability');
  if (memResult?.passed) {
    console.log('✅ Memory: No leaks detected');
  }

  // Zero regressions
  const regressionResult = results.find(r => r.name === 'Zero Regression Check');
  if (regressionResult?.passed) {
    console.log('✅ Regressions: Zero detected - all APIs functional');
  }

  console.log();
  console.log('='.repeat(70));

  if (failed === 0) {
    console.log('\n🎉 All E2E tests PASSED! WASM integration fully functional.\n');
    return 0;
  } else {
    console.log('\n❌ Some E2E tests FAILED. Review errors above.\n');
    return 1;
  }
}

// Execute test suite
runE2ETests()
  .then(exitCode => process.exit(exitCode))
  .catch(error => {
    console.error('\n💥 Fatal error running E2E tests:', error);
    process.exit(1);
  });
