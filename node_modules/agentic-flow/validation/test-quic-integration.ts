// QUIC Integration Validation Tests
import { QuicClient, QuicServer, QuicConnectionPool } from '../src/transport/quic.js';
import { loadQuicConfig, checkQuicAvailability } from '../src/config/quic.js';
import { logger } from '../src/utils/logger.js';

interface TestResult {
  name: string;
  passed: boolean;
  error?: string;
  duration?: number;
}

const results: TestResult[] = [];

async function runTest(name: string, fn: () => Promise<void>): Promise<void> {
  const startTime = Date.now();
  try {
    await fn();
    results.push({
      name,
      passed: true,
      duration: Date.now() - startTime
    });
    console.log(`✅ ${name} (${Date.now() - startTime}ms)`);
  } catch (error) {
    results.push({
      name,
      passed: false,
      error: (error as Error).message,
      duration: Date.now() - startTime
    });
    console.error(`❌ ${name}: ${(error as Error).message}`);
  }
}

async function testQuicAvailability(): Promise<void> {
  const availability = await checkQuicAvailability();
  if (availability.available) {
    logger.info('QUIC is available');
  } else {
    logger.warn('QUIC is not available', { reason: availability.reason });
  }
}

async function testConfigLoading(): Promise<void> {
  const config = loadQuicConfig({
    enabled: true,
    port: 4433,
    maxConnections: 50
  });

  if (config.port !== 4433) {
    throw new Error('Config port mismatch');
  }

  if (config.maxConnections !== 50) {
    throw new Error('Config maxConnections mismatch');
  }

  logger.info('Config loaded successfully', { config });
}

async function testQuicClientInitialization(): Promise<void> {
  const client = new QuicClient({
    serverHost: 'localhost',
    serverPort: 4433
  });

  await client.initialize();
  const stats = client.getStats();

  if (typeof stats.totalConnections !== 'number') {
    throw new Error('Stats not properly initialized');
  }

  await client.shutdown();
  logger.info('Client initialization successful');
}

async function testQuicServerInitialization(): Promise<void> {
  const server = new QuicServer({
    host: '127.0.0.1',
    port: 14433, // Use non-standard port for testing
    certPath: './certs/cert.pem',
    keyPath: './certs/key.pem'
  });

  await server.initialize();
  const stats = server.getStats();

  if (typeof stats.totalConnections !== 'number') {
    throw new Error('Server stats not properly initialized');
  }

  logger.info('Server initialization successful');
}

async function testConnectionPool(): Promise<void> {
  const client = new QuicClient();
  await client.initialize();

  const pool = new QuicConnectionPool(client, 5);

  // Get connection (should create new)
  const conn1 = await pool.getConnection('localhost', 4433);

  // Get same connection (should reuse)
  const conn2 = await pool.getConnection('localhost', 4433);

  if (conn1.id !== conn2.id) {
    throw new Error('Connection pool not reusing connections');
  }

  await pool.clear();
  await client.shutdown();

  logger.info('Connection pool test successful');
}

async function testConfigValidation(): Promise<void> {
  try {
    loadQuicConfig({
      enabled: true,
      port: 99999, // Invalid port
      maxConnections: 100
    });
    throw new Error('Should have thrown validation error for invalid port');
  } catch (error) {
    if ((error as Error).message.includes('Invalid port')) {
      logger.info('Config validation working correctly');
    } else {
      throw error;
    }
  }
}

async function testHealthCheck(): Promise<void> {
  const availability = await checkQuicAvailability();

  if (typeof availability.available !== 'boolean') {
    throw new Error('Health check not returning proper status');
  }

  logger.info('Health check successful', { availability });
}

async function testStatsCollection(): Promise<void> {
  const client = new QuicClient();
  await client.initialize();

  const stats = client.getStats();

  const requiredFields = [
    'totalConnections',
    'activeConnections',
    'totalStreams',
    'activeStreams',
    'bytesReceived',
    'bytesSent',
    'packetsLost',
    'rttMs'
  ];

  for (const field of requiredFields) {
    if (!(field in stats)) {
      throw new Error(`Missing stats field: ${field}`);
    }
  }

  await client.shutdown();
  logger.info('Stats collection successful', { stats });
}

async function testEnvironmentVariables(): Promise<void> {
  const originalValue = process.env.AGENTIC_FLOW_ENABLE_QUIC;

  // Test with enabled
  process.env.AGENTIC_FLOW_ENABLE_QUIC = 'true';
  let config = loadQuicConfig();
  if (!config.enabled) {
    throw new Error('Environment variable not enabling QUIC');
  }

  // Test with disabled
  process.env.AGENTIC_FLOW_ENABLE_QUIC = 'false';
  config = loadQuicConfig();
  if (config.enabled) {
    throw new Error('Environment variable not disabling QUIC');
  }

  // Restore
  if (originalValue !== undefined) {
    process.env.AGENTIC_FLOW_ENABLE_QUIC = originalValue;
  } else {
    delete process.env.AGENTIC_FLOW_ENABLE_QUIC;
  }

  logger.info('Environment variable handling successful');
}

async function main(): Promise<void> {
  console.log('\n🧪 QUIC Integration Tests\n');
  console.log('='.repeat(50));

  await runTest('QUIC Availability Check', testQuicAvailability);
  await runTest('Configuration Loading', testConfigLoading);
  await runTest('Configuration Validation', testConfigValidation);
  await runTest('Client Initialization', testQuicClientInitialization);
  await runTest('Server Initialization', testQuicServerInitialization);
  await runTest('Connection Pool', testConnectionPool);
  await runTest('Health Check', testHealthCheck);
  await runTest('Stats Collection', testStatsCollection);
  await runTest('Environment Variables', testEnvironmentVariables);

  console.log('\n' + '='.repeat(50));
  console.log('\n📊 Test Summary:\n');

  const passed = results.filter(r => r.passed).length;
  const failed = results.filter(r => !r.passed).length;
  const totalDuration = results.reduce((sum, r) => sum + (r.duration || 0), 0);

  console.log(`Total: ${results.length}`);
  console.log(`Passed: ${passed}`);
  console.log(`Failed: ${failed}`);
  console.log(`Duration: ${totalDuration}ms`);

  if (failed > 0) {
    console.log('\n❌ Failed Tests:\n');
    results.filter(r => !r.passed).forEach(r => {
      console.log(`  • ${r.name}: ${r.error}`);
    });
    process.exit(1);
  } else {
    console.log('\n✅ All tests passed!\n');
    process.exit(0);
  }
}

// Run tests
main().catch(error => {
  console.error('Test suite failed:', error);
  process.exit(1);
});
