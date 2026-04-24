#!/usr/bin/env node
/**
 * Regression Test Suite for agentic-flow v1.5.13
 *
 * Tests to ensure no regressions were introduced by:
 * - Backend selector implementation
 * - Package exports updates
 * - Documentation changes
 */

import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { existsSync } from 'fs';
import { createRequire } from 'module';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const require = createRequire(import.meta.url);

console.log('🧪 Regression Test Suite - agentic-flow v1.5.13\n');
console.log('━'.repeat(60) + '\n');

let passed = 0;
let failed = 0;

function test(name, fn) {
  try {
    fn();
    console.log(`✅ ${name}`);
    passed++;
  } catch (error) {
    console.error(`❌ ${name}`);
    console.error(`   Error: ${error.message}`);
    failed++;
  }
}

async function testAsync(name, fn) {
  try {
    await fn();
    console.log(`✅ ${name}`);
    passed++;
  } catch (error) {
    console.error(`❌ ${name}`);
    console.error(`   Error: ${error.message}`);
    failed++;
  }
}

// Test 1: Backend Selector Module Exports
console.log('📦 Test Group 1: Backend Selector Module\n');

await testAsync('Import backend-selector module', async () => {
  const module = await import('../dist/reasoningbank/backend-selector.js');
  if (!module.getRecommendedBackend) throw new Error('Missing getRecommendedBackend');
  if (!module.createOptimalReasoningBank) throw new Error('Missing createOptimalReasoningBank');
  if (!module.validateEnvironment) throw new Error('Missing validateEnvironment');
});

await testAsync('getRecommendedBackend() returns valid backend', async () => {
  const { getRecommendedBackend } = await import('../dist/reasoningbank/backend-selector.js');
  const backend = getRecommendedBackend();
  if (backend !== 'nodejs' && backend !== 'wasm') {
    throw new Error(`Invalid backend: ${backend}`);
  }
});

await testAsync('getBackendInfo() returns valid structure', async () => {
  const { getBackendInfo } = await import('../dist/reasoningbank/backend-selector.js');
  const info = getBackendInfo();
  if (!info.backend) throw new Error('Missing backend field');
  if (!info.environment) throw new Error('Missing environment field');
  if (!info.features) throw new Error('Missing features field');
  if (!info.storage) throw new Error('Missing storage field');
});

await testAsync('validateEnvironment() returns validation object', async () => {
  const { validateEnvironment } = await import('../dist/reasoningbank/backend-selector.js');
  const result = validateEnvironment();
  if (typeof result.valid !== 'boolean') throw new Error('Missing valid field');
  if (!Array.isArray(result.warnings)) throw new Error('Warnings should be array');
  if (!result.backend) throw new Error('Missing backend field');
});

console.log('');

// Test 2: ReasoningBank Core Module (Node.js backend)
console.log('📦 Test Group 2: ReasoningBank Core Module\n');

await testAsync('Import reasoningbank index module', async () => {
  const module = await import('../dist/reasoningbank/index.js');
  if (!module.initialize) throw new Error('Missing initialize function');
  if (!module.db) throw new Error('Missing db module');
  if (!module.retrieveMemories) throw new Error('Missing retrieveMemories');
});

await testAsync('db module has required functions', async () => {
  const { db } = await import('../dist/reasoningbank/index.js');
  if (!db.runMigrations) throw new Error('Missing runMigrations');
  if (!db.getDb) throw new Error('Missing getDb');
  if (!db.fetchMemoryCandidates) throw new Error('Missing fetchMemoryCandidates');
});

console.log('');

// Test 3: WASM Adapter Module
console.log('📦 Test Group 3: WASM Adapter Module (Note: WASM requires --experimental-wasm-modules)\n');

test('WASM adapter file exists', () => {
  if (!existsSync('./dist/reasoningbank/wasm-adapter.js')) {
    throw new Error('wasm-adapter.js missing');
  }
});

test('WASM binary exists', () => {
  if (!existsSync('./wasm/reasoningbank/reasoningbank_wasm_bg.wasm')) {
    throw new Error('WASM binary missing');
  }
});

console.log('   ℹ️  WASM import tests skipped (require --experimental-wasm-modules flag)');

console.log('');

// Test 4: Package Exports Resolution
console.log('📦 Test Group 4: Package Exports\n');

await testAsync('Main export resolves', async () => {
  try {
    // This will fail without Claude Code, but should resolve
    await import('../dist/index.js');
  } catch (error) {
    // Expected to fail in test environment, just check it resolves
    if (error.code === 'ERR_MODULE_NOT_FOUND') throw error;
    // Other errors are OK (e.g., missing Claude Code binary)
  }
});

await testAsync('reasoningbank export resolves (Node.js)', async () => {
  // In Node.js, should resolve to index.js
  const module = await import('../dist/reasoningbank/index.js');
  if (!module.db) throw new Error('Should have db module in Node.js');
});

await testAsync('backend-selector export resolves', async () => {
  const module = await import('../dist/reasoningbank/backend-selector.js');
  if (!module.createOptimalReasoningBank) throw new Error('Missing main function');
});

test('wasm-adapter export path exists', () => {
  if (!existsSync('./dist/reasoningbank/wasm-adapter.js')) {
    throw new Error('wasm-adapter.js not found');
  }
});

console.log('');

// Test 5: Backward Compatibility
console.log('📦 Test Group 5: Backward Compatibility\n');

await testAsync('Old import path still works', async () => {
  // Old: import from dist/reasoningbank/index.js
  const module = await import('../dist/reasoningbank/index.js');
  if (!module.initialize) throw new Error('Old import broken');
});

await testAsync('Core functions unchanged', async () => {
  const { retrieveMemories, judgeTrajectory, distillMemories, consolidate } =
    await import('../dist/reasoningbank/index.js');

  if (typeof retrieveMemories !== 'function') throw new Error('retrieveMemories broken');
  if (typeof judgeTrajectory !== 'function') throw new Error('judgeTrajectory broken');
  if (typeof distillMemories !== 'function') throw new Error('distillMemories broken');
  if (typeof consolidate !== 'function') throw new Error('consolidate broken');
});

test('WASM adapter file integrity', () => {
  const { readFileSync } = require('fs');
  const content = readFileSync('./dist/reasoningbank/wasm-adapter.js', 'utf-8');
  if (!content.includes('createReasoningBank')) {
    throw new Error('createReasoningBank function missing from file');
  }
  if (!content.includes('ReasoningBankAdapter')) {
    throw new Error('ReasoningBankAdapter class missing from file');
  }
});

console.log('');

// Test 6: Router Module (should be untouched)
console.log('📦 Test Group 6: Other Modules (Router)\n');

await testAsync('Router module still works', async () => {
  const module = await import('../dist/router/router.js');
  if (!module.ModelRouter) throw new Error('ModelRouter missing');
});

console.log('');

// Test 7: File Structure Integrity
console.log('📦 Test Group 7: File Structure\n');

test('backend-selector.js exists', () => {
  if (!existsSync('./dist/reasoningbank/backend-selector.js')) {
    throw new Error('backend-selector.js not in dist/');
  }
});

test('index.js exists', () => {
  if (!existsSync('./dist/reasoningbank/index.js')) {
    throw new Error('index.js not in dist/');
  }
});

test('wasm-adapter.js exists', () => {
  if (!existsSync('./dist/reasoningbank/wasm-adapter.js')) {
    throw new Error('wasm-adapter.js not in dist/');
  }
});

test('WASM files exist', () => {
  if (!existsSync('./wasm/reasoningbank/reasoningbank_wasm.js')) {
    throw new Error('WASM JS not found');
  }
  if (!existsSync('./wasm/reasoningbank/reasoningbank_wasm_bg.wasm')) {
    throw new Error('WASM binary not found');
  }
});

console.log('');

// Summary
console.log('━'.repeat(60));
console.log('📊 REGRESSION TEST SUMMARY');
console.log('━'.repeat(60) + '\n');
console.log(`Total Tests: ${passed + failed}`);
console.log(`✅ Passed: ${passed}`);
console.log(`❌ Failed: ${failed}\n`);

if (failed === 0) {
  console.log('🎉 All regression tests passed! No regressions detected.\n');
  process.exit(0);
} else {
  console.log('❌ Some regression tests failed. Review output above.\n');
  process.exit(1);
}
