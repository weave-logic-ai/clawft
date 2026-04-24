/**
 * QUIC WASM Integration Test
 *
 * Validates that the QUIC transport WASM module loads correctly
 * and provides the expected API surface.
 */

import { existsSync } from 'fs';
import { join } from 'path';

async function testQuicWasmIntegration() {
  console.log('🧪 Testing QUIC WASM Integration...\n');

  try {
    // Test 1: Load WASM module
    console.log('1️⃣  Loading WASM module...');

    // Try both production and development paths
    const prodPath = join(process.cwd(), 'wasm/quic/agentic_flow_quic.js');
    const devPath = join(process.cwd(), '../crates/agentic-flow-quic/pkg/agentic_flow_quic.js');

    let wasmPath: string;
    if (existsSync(prodPath)) {
      wasmPath = prodPath;
    } else if (existsSync(devPath)) {
      wasmPath = devPath;
    } else {
      throw new Error(`WASM module not found at ${prodPath} or ${devPath}`);
    }

    console.log(`   Loading from: ${wasmPath}`);
    const wasm = await import(wasmPath);
    console.log('   ✅ WASM module loaded successfully');

    // Test 2: Check exports
    console.log('\n2️⃣  Checking exports...');
    const requiredExports = [
      'WasmQuicClient',
      'createQuicMessage',
      'defaultConfig'
    ];

    for (const exportName of requiredExports) {
      if (wasm[exportName]) {
        console.log(`   ✅ ${exportName} exported`);
      } else {
        throw new Error(`Missing export: ${exportName}`);
      }
    }

    // Test 3: Create default config
    console.log('\n3️⃣  Creating default config...');
    const config = wasm.defaultConfig();
    console.log('   ✅ Default config created:', config);

    // Test 4: Create QUIC message
    console.log('\n4️⃣  Creating QUIC message...');
    const message = wasm.createQuicMessage(
      'test-msg-1',
      'task',
      new TextEncoder().encode(JSON.stringify({ action: 'test' })),
      null
    );
    console.log('   ✅ QUIC message created:', message);

    console.log('\n✅ All QUIC WASM integration tests passed!\n');
    console.log('📊 Summary:');
    console.log('   - WASM module: loaded');
    console.log('   - Exports: verified');
    console.log('   - Config creation: working');
    console.log('   - Message creation: working');
    console.log('\n⚠️  Note: This WASM build uses stubs. For full QUIC functionality, use native Node.js builds.\n');

    return { success: true };

  } catch (error) {
    console.error('\n❌ QUIC WASM integration test failed:', error);
    if (error instanceof Error) {
      console.error('   Error:', error.message);
      console.error('   Stack:', error.stack);
    }
    return { success: false, error };
  }
}

// Run tests
testQuicWasmIntegration()
  .then(result => {
    process.exit(result.success ? 0 : 1);
  })
  .catch(err => {
    console.error('Unexpected error:', err);
    process.exit(1);
  });
