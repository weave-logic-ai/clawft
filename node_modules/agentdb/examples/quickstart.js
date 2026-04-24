/**
 * AgentDB Quickstart Example
 *
 * This example shows the recommended way to use AgentDB programmatically.
 *
 * Usage:
 *   node examples/quickstart.js
 */

import { AgentDB } from 'agentdb';
import { createDatabase } from 'agentdb';

async function main() {
  console.log('=== AgentDB Quickstart Example ===\n');

  try {
    // Method 1: Simple initialization
    console.log('1. Creating AgentDB instance...');
    const db = await createDatabase('./examples/quickstart.db');

    // TODO: Auto-initialize schemas in future version
    // For now, you need to run: agentdb init first
    // This will be fixed in alpha.3

    console.log('✓ Database created\n');

    // For now, recommend using CLI for initialization:
    console.log('Note: For alpha.2, please initialize via CLI first:');
    console.log('  npx agentdb init --db ./examples/quickstart.db\n');

    // Display package version
    const packageJson = await import('agentdb/package.json', {
      assert: { type: 'json' }
    });
    console.log(`AgentDB version: ${packageJson.default.version}`);

  } catch (error) {
    console.error('Error:', error.message);
    process.exit(1);
  }
}

main();
