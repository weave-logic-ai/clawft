/**
 * QUIC Synchronization Example
 *
 * Demonstrates how to use QUICServer, QUICClient, and SyncCoordinator
 * for bidirectional synchronization between AgentDB instances.
 */

import { createDatabase } from '../db-fallback.js';
import {
  QUICServer,
  QUICClient,
  SyncCoordinator,
  ReflexionMemory,
  SkillLibrary,
  EmbeddingService,
} from '../controllers/index.js';

async function exampleQUICSync() {
  console.log('🚀 QUIC Synchronization Example\n');

  // Initialize two database instances (simulating local and remote)
  const localDB = await createDatabase('./local-agent.db');
  const remoteDB = await createDatabase('./remote-agent.db');

  // Initialize embedding services
  const localEmbedder = new EmbeddingService({
    model: 'Xenova/all-MiniLM-L6-v2',
    dimension: 384,
    provider: 'transformers',
  });
  const remoteEmbedder = new EmbeddingService({
    model: 'Xenova/all-MiniLM-L6-v2',
    dimension: 384,
    provider: 'transformers',
  });

  // Initialize memory controllers
  const localReflexion = new ReflexionMemory(localDB, localEmbedder);
  const remoteReflexion = new ReflexionMemory(remoteDB, remoteEmbedder);
  const remoteSkillLib = new SkillLibrary(remoteDB, remoteEmbedder);

  try {
    // ===== Setup Remote Server =====
    console.log('📡 Setting up remote QUIC server...');
    const server = new QUICServer(remoteDB, {
      host: '0.0.0.0',
      port: 4433,
      authToken: 'secret-token-123',
      maxConnections: 10,
      rateLimit: {
        maxRequestsPerMinute: 60,
        maxBytesPerMinute: 10 * 1024 * 1024,
      },
    });

    await server.start();
    console.log('✓ Server started\n');

    // ===== Setup Local Client =====
    console.log('📱 Setting up local QUIC client...');
    const client = new QUICClient({
      serverHost: 'localhost',
      serverPort: 4433,
      authToken: 'secret-token-123',
      maxRetries: 3,
      retryDelayMs: 1000,
      timeoutMs: 30000,
      poolSize: 5,
    });

    await client.connect();
    console.log('✓ Client connected\n');

    // Test ping
    console.log('🏓 Testing connection...');
    const pingResult = await client.ping();
    console.log(`✓ Ping: ${pingResult.latencyMs}ms\n`);

    // ===== Add some data to remote =====
    console.log('📝 Adding test data to remote database...');
    await remoteReflexion.storeEpisode({
      sessionId: 'test-session',
      task: 'Calculate fibonacci',
      input: 'n=10',
      output: '55',
      critique: 'Efficient implementation',
      reward: 1.0,
      success: true,
      latencyMs: 150,
      tokensUsed: 200,
      tags: ['math', 'algorithm'],
    });

    await remoteSkillLib.createSkill({
      name: 'fibonacci',
      description: 'Calculate fibonacci numbers',
      signature: {
        inputs: { n: 'number' },
        outputs: { result: 'number' },
      },
      code: 'function fib(n) { ... }',
      successRate: 1.0,
      uses: 5,
      avgReward: 0.95,
      avgLatencyMs: 100,
    });
    console.log('✓ Test data added\n');

    // ===== Setup Sync Coordinator =====
    console.log('🔄 Setting up sync coordinator...');
    const coordinator = new SyncCoordinator({
      db: localDB,
      client: client,
      server: server,
      conflictStrategy: 'latest-wins',
      batchSize: 100,
      autoSync: false,
    });

    // ===== Perform Manual Sync =====
    console.log('🔄 Starting synchronization...\n');
    const syncReport = await coordinator.sync((progress) => {
      console.log(`  [${progress.phase}] ${progress.message || ''}`);
      if (progress.itemType) {
        console.log(`    Syncing ${progress.itemType}: ${progress.current}/${progress.total}`);
      }
    });

    console.log('\n📊 Sync Report:');
    console.log(`  Success: ${syncReport.success}`);
    console.log(`  Duration: ${syncReport.durationMs}ms`);
    console.log(`  Items pulled: ${syncReport.itemsPulled}`);
    console.log(`  Items pushed: ${syncReport.itemsPushed}`);
    console.log(`  Conflicts resolved: ${syncReport.conflictsResolved}`);
    console.log(`  Bytes transferred: ${syncReport.bytesTransferred}`);
    if (syncReport.errors.length > 0) {
      console.log(`  Errors: ${syncReport.errors.join(', ')}`);
    }

    // ===== Check Sync State =====
    console.log('\n📊 Sync State:');
    const state = coordinator.getSyncState();
    console.log(`  Last sync: ${new Date(state.lastSyncAt).toISOString()}`);
    console.log(`  Total items synced: ${state.totalItemsSynced}`);
    console.log(`  Total bytes synced: ${state.totalBytesSynced}`);
    console.log(`  Sync count: ${state.syncCount}`);

    // ===== Server Status =====
    console.log('\n📡 Server Status:');
    const serverStatus = server.getStatus();
    console.log(`  Running: ${serverStatus.isRunning}`);
    console.log(`  Active connections: ${serverStatus.activeConnections}`);
    console.log(`  Total requests: ${serverStatus.totalRequests}`);

    // ===== Client Status =====
    console.log('\n📱 Client Status:');
    const clientStatus = client.getStatus();
    console.log(`  Connected: ${clientStatus.isConnected}`);
    console.log(`  Pool size: ${clientStatus.poolSize}`);
    console.log(`  Active connections: ${clientStatus.activeConnections}`);
    console.log(`  Total requests: ${clientStatus.totalRequests}`);

    // ===== Enable Auto-Sync =====
    console.log('\n🔄 Testing auto-sync...');
    coordinator['config'].autoSync = true;
    coordinator['config'].syncIntervalMs = 5000; // 5 seconds
    coordinator['startAutoSync']();
    console.log('✓ Auto-sync enabled (5s interval)');

    // Wait for one auto-sync cycle
    await new Promise((resolve) => setTimeout(resolve, 6000));
    coordinator.stopAutoSync();
    console.log('✓ Auto-sync disabled');

    // ===== Cleanup =====
    console.log('\n🧹 Cleaning up...');
    await client.disconnect();
    await server.stop();
    console.log('✓ Cleanup complete');

    console.log('\n✅ QUIC Sync example completed successfully!');
  } catch (error) {
    console.error('\n❌ Error:', error);
    throw error;
  }
}

// Run example if executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  exampleQUICSync()
    .then(() => process.exit(0))
    .catch((error) => {
      console.error('Fatal error:', error);
      process.exit(1);
    });
}

export { exampleQUICSync };
