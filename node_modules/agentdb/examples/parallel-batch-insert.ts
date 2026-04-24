/**
 * Example: Parallel Batch Insert Optimization
 *
 * Demonstrates the 3-5x speedup achieved with parallel batch inserts
 * compared to sequential inserts.
 */

import Database from 'better-sqlite3';
import { BatchOperations } from '../src/optimizations/BatchOperations.js';
import { EmbeddingService } from '../src/controllers/EmbeddingService.js';
import * as fs from 'fs';
import * as path from 'path';

async function main() {
  // Initialize database
  const db = new Database('./examples/data/parallel-demo.db');
  db.pragma('journal_mode = WAL');

  // Load schemas
  const schemaPath = path.join(__dirname, '../src/schemas/schema.sql');
  if (fs.existsSync(schemaPath)) {
    db.exec(fs.readFileSync(schemaPath, 'utf-8'));
  }

  // Initialize embedder
  const embedder = new EmbeddingService({
    model: 'mock-model',
    dimension: 384,
    provider: 'local',
  });
  await embedder.initialize();

  // Initialize batch operations
  const batchOps = new BatchOperations(db, embedder, {
    batchSize: 100,
    parallelism: 4,
    progressCallback: (completed, total) => {
      const percent = ((completed / total) * 100).toFixed(1);
      console.log(`Progress: ${completed}/${total} (${percent}%)`);
    },
  });

  console.log('🚀 Parallel Batch Insert Demo\n');

  // Generate test data
  const dataSize = 10000;
  const testData = Array.from({ length: dataSize }, (_, i) => ({
    session_id: `session-${i}`,
    task: `Task ${i}: Process data with complex operations`,
    input: `Input data for task ${i}`,
    output: `Output result for task ${i}`,
    critique: i % 10 === 0 ? `Performance could be improved` : null,
    reward: Math.random(),
    success: Math.random() > 0.2 ? 1 : 0,
    latency_ms: Math.floor(Math.random() * 1000),
    tokens_used: Math.floor(Math.random() * 500),
  }));

  console.log(`Generated ${dataSize} rows of test data\n`);

  // Sequential insert benchmark
  console.log('📊 Sequential Insert (baseline):');
  const seqStart = Date.now();

  const stmt = db.prepare(`
    INSERT INTO episodes (
      session_id, task, input, output, critique,
      reward, success, latency_ms, tokens_used
    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
  `);

  const seqTransaction = db.transaction(() => {
    testData.forEach((row) => {
      stmt.run(
        row.session_id,
        row.task,
        row.input,
        row.output,
        row.critique,
        row.reward,
        row.success,
        row.latency_ms,
        row.tokens_used
      );
    });
  });

  seqTransaction();
  const seqDuration = Date.now() - seqStart;

  console.log(`  Time: ${seqDuration}ms`);
  console.log(`  Rate: ${Math.floor(dataSize / (seqDuration / 1000))} rows/sec\n`);

  // Clean up for parallel test
  db.exec('DELETE FROM episodes');

  // Parallel insert benchmark
  console.log('⚡ Parallel Insert (chunkSize: 1000, maxConcurrency: 5):');

  const result = await batchOps.batchInsertParallel(
    'episodes',
    testData,
    [
      'session_id',
      'task',
      'input',
      'output',
      'critique',
      'reward',
      'success',
      'latency_ms',
      'tokens_used',
    ],
    {
      chunkSize: 1000,
      maxConcurrency: 5,
      useTransaction: true,
      retryAttempts: 3,
    }
  );

  console.log(`\n📈 Results:`);
  console.log(`  Total inserted: ${result.totalInserted} rows`);
  console.log(`  Chunks processed: ${result.chunksProcessed}`);
  console.log(`  Time: ${result.duration}ms`);
  console.log(`  Rate: ${Math.floor(result.totalInserted / (result.duration / 1000))} rows/sec`);
  console.log(`  Errors: ${result.errors.length}`);
  console.log(`  Speedup: ${(seqDuration / result.duration).toFixed(2)}x faster\n`);

  // Different concurrency levels
  console.log('🔬 Testing Different Concurrency Levels:\n');

  const concurrencyLevels = [1, 2, 5, 10];

  for (const maxConcurrency of concurrencyLevels) {
    db.exec('DELETE FROM episodes');

    const testResult = await batchOps.batchInsertParallel(
      'episodes',
      testData,
      [
        'session_id',
        'task',
        'input',
        'output',
        'critique',
        'reward',
        'success',
        'latency_ms',
        'tokens_used',
      ],
      {
        chunkSize: 1000,
        maxConcurrency,
        useTransaction: true,
      }
    );

    console.log(
      `  Concurrency ${maxConcurrency}: ${testResult.duration}ms (${(
        seqDuration / testResult.duration
      ).toFixed(2)}x speedup)`
    );
  }

  console.log('\n✅ Demo completed successfully!');

  // Cleanup
  db.close();
  if (fs.existsSync('./examples/data/parallel-demo.db')) {
    fs.unlinkSync('./examples/data/parallel-demo.db');
  }
}

main().catch(console.error);
