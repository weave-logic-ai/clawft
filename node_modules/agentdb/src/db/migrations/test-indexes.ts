#!/usr/bin/env node
/**
 * AgentDB Index Performance Test
 *
 * Tests composite index performance by:
 * 1. Creating a test database with sample data
 * 2. Running queries WITHOUT composite indexes
 * 3. Applying composite index migration
 * 4. Running same queries WITH composite indexes
 * 5. Comparing performance metrics
 *
 * Usage:
 *   node test-indexes.ts
 */

import Database from 'better-sqlite3';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

interface BenchmarkResult {
  query: string;
  description: string;
  timeMs: number;
  rowsReturned: number;
  indexUsed: string | null;
}

interface BenchmarkSuite {
  before: BenchmarkResult[];
  after: BenchmarkResult[];
  improvement: {
    avgSpeedup: number;
    totalTimeBefore: number;
    totalTimeAfter: number;
  };
}

/**
 * Create test database with sample data
 */
function createTestDatabase(): Database.Database {
  const dbPath = path.join(__dirname, 'test-performance.db');

  // Remove existing test database
  if (fs.existsSync(dbPath)) {
    fs.unlinkSync(dbPath);
  }

  const db = new Database(dbPath);
  db.pragma('foreign_keys = ON');

  console.log('📦 Creating test database with sample data...\n');

  // Load base schema
  const schemaPath = path.resolve(__dirname, '../../schemas/schema.sql');
  const schema = fs.readFileSync(schemaPath, 'utf-8');
  db.exec(schema);

  // Load frontier schema
  const frontierPath = path.resolve(__dirname, '../../schemas/frontier-schema.sql');
  const frontierSchema = fs.readFileSync(frontierPath, 'utf-8');
  db.exec(frontierSchema);

  // Generate sample data
  console.log('   Generating episodes...');
  const insertEpisode = db.prepare(`
    INSERT INTO episodes (session_id, task, input, output, critique, reward, success, latency_ms, tokens_used)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
  `);

  const sessions = 100;
  const episodesPerSession = 50;

  for (let s = 0; s < sessions; s++) {
    const sessionId = `session-${s}`;
    for (let e = 0; e < episodesPerSession; e++) {
      insertEpisode.run(
        sessionId,
        `Task ${e % 10}`,
        `Input ${e}`,
        `Output ${e}`,
        `Critique ${e}`,
        Math.random(),
        Math.random() > 0.5 ? 1 : 0,
        Math.floor(Math.random() * 1000),
        Math.floor(Math.random() * 5000)
      );
    }
  }

  console.log(`   ✅ Created ${sessions * episodesPerSession} episodes`);

  // Generate skills
  console.log('   Generating skills...');
  const insertSkill = db.prepare(`
    INSERT INTO skills (name, description, signature, success_rate, uses, avg_reward, avg_latency_ms)
    VALUES (?, ?, ?, ?, ?, ?, ?)
  `);

  for (let i = 0; i < 500; i++) {
    insertSkill.run(
      `Skill ${i}`,
      `Description for skill ${i}`,
      JSON.stringify({ inputs: {}, outputs: {} }),
      Math.random(),
      Math.floor(Math.random() * 100),
      Math.random(),
      Math.floor(Math.random() * 500)
    );
  }

  console.log(`   ✅ Created 500 skills`);

  // Generate reasoning patterns
  console.log('   Generating reasoning patterns...');
  const insertPattern = db.prepare(`
    INSERT INTO reasoning_patterns (task_type, approach, success_rate, uses, avg_reward)
    VALUES (?, ?, ?, ?, ?)
  `);

  const taskTypes = ['coding', 'analysis', 'planning', 'debugging', 'optimization'];
  for (let i = 0; i < 1000; i++) {
    insertPattern.run(
      taskTypes[i % taskTypes.length],
      `Approach ${i}`,
      Math.random(),
      Math.floor(Math.random() * 50),
      Math.random()
    );
  }

  console.log(`   ✅ Created 1000 reasoning patterns`);

  // Generate causal edges
  console.log('   Generating causal edges...');
  const insertEdge = db.prepare(`
    INSERT INTO causal_edges (from_memory_id, from_memory_type, to_memory_id, to_memory_type, similarity, uplift, confidence, sample_size)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?)
  `);

  for (let i = 0; i < 2000; i++) {
    insertEdge.run(
      Math.floor(Math.random() * 5000) + 1,
      'episode',
      Math.floor(Math.random() * 5000) + 1,
      'episode',
      Math.random(),
      Math.random() * 2 - 1,
      Math.random(),
      Math.floor(Math.random() * 100)
    );
  }

  console.log(`   ✅ Created 2000 causal edges\n`);

  return db;
}

/**
 * Run query and measure performance
 */
function benchmarkQuery(
  db: Database.Database,
  query: string,
  params: any[],
  description: string
): BenchmarkResult {
  const startTime = performance.now();

  const stmt = db.prepare(query);
  const rows = params.length > 0 ? stmt.all(...params) : stmt.all();

  const endTime = performance.now();
  const timeMs = endTime - startTime;

  // Get query plan to check index usage
  const explainStmt = db.prepare(`EXPLAIN QUERY PLAN ${query}`);
  const plan = params.length > 0 ? explainStmt.all(...params) : explainStmt.all();

  // Extract index usage from plan
  const planText = plan.map((p: any) => p.detail).join(' ');
  const indexMatch = planText.match(/USING INDEX ([a-z_0-9]+)/i);
  const indexUsed = indexMatch ? indexMatch[1] : null;

  return {
    query,
    description,
    timeMs: parseFloat(timeMs.toFixed(3)),
    rowsReturned: rows.length,
    indexUsed,
  };
}

/**
 * Run benchmark suite
 */
function runBenchmarks(db: Database.Database): BenchmarkResult[] {
  console.log('🔍 Running query benchmarks...\n');

  const benchmarks: BenchmarkResult[] = [];

  // Benchmark 1: Episode query with session_id and ORDER BY ts
  benchmarks.push(
    benchmarkQuery(
      db,
      'SELECT * FROM episodes WHERE session_id = ? ORDER BY ts DESC LIMIT 10',
      ['session-0'],
      'Episode query: session_id + ORDER BY ts DESC'
    )
  );

  // Benchmark 2: Episode query with session_id and task
  benchmarks.push(
    benchmarkQuery(
      db,
      'SELECT * FROM episodes WHERE session_id = ? AND task = ? ORDER BY reward DESC',
      ['session-0', 'Task 5'],
      'Episode query: session_id + task + ORDER BY reward'
    )
  );

  // Benchmark 3: Skills ordered by success_rate and uses
  benchmarks.push(
    benchmarkQuery(
      db,
      'SELECT * FROM skills ORDER BY success_rate DESC, uses DESC LIMIT 10',
      [],
      'Skills query: ORDER BY success_rate DESC, uses DESC'
    )
  );

  // Benchmark 4: Reasoning patterns by task_type and success_rate
  benchmarks.push(
    benchmarkQuery(
      db,
      'SELECT * FROM reasoning_patterns WHERE task_type = ? ORDER BY success_rate DESC LIMIT 10',
      ['coding'],
      'Patterns query: task_type + ORDER BY success_rate'
    )
  );

  // Benchmark 5: Causal edges lookup
  benchmarks.push(
    benchmarkQuery(
      db,
      'SELECT * FROM causal_edges WHERE from_memory_id = ? AND to_memory_id = ?',
      [100, 200],
      'Causal edges: from_memory_id + to_memory_id'
    )
  );

  // Benchmark 6: Causal edges by confidence
  benchmarks.push(
    benchmarkQuery(
      db,
      'SELECT * FROM causal_edges WHERE from_memory_id = ? ORDER BY similarity DESC LIMIT 10',
      [100],
      'Causal edges: from_memory_id + ORDER BY similarity'
    )
  );

  // Benchmark 7: Episode with task and success
  benchmarks.push(
    benchmarkQuery(
      db,
      'SELECT * FROM episodes WHERE task = ? AND success = 1 ORDER BY reward DESC LIMIT 10',
      ['Task 5'],
      'Episodes: task + success + ORDER BY reward'
    )
  );

  // Benchmark 8: Skills with success_rate threshold
  benchmarks.push(
    benchmarkQuery(
      db,
      'SELECT * FROM skills WHERE success_rate >= 0.7 ORDER BY avg_reward DESC LIMIT 10',
      [],
      'Skills: success_rate threshold + ORDER BY avg_reward'
    )
  );

  return benchmarks;
}

/**
 * Display benchmark results
 */
function displayResults(results: BenchmarkResult[], title: string) {
  console.log(`\n${title}`);
  console.log('='.repeat(100));
  console.log();

  results.forEach((result, idx) => {
    console.log(`${idx + 1}. ${result.description}`);
    console.log(`   Time: ${result.timeMs}ms`);
    console.log(`   Rows: ${result.rowsReturned}`);
    console.log(`   Index: ${result.indexUsed || 'NONE (table scan)'}`);
    console.log();
  });
}

/**
 * Compare before/after results
 */
function compareResults(suite: BenchmarkSuite) {
  console.log('\n📊 Performance Comparison');
  console.log('='.repeat(100));
  console.log();

  suite.before.forEach((before, idx) => {
    const after = suite.after[idx];
    const speedup = before.timeMs / after.timeMs;
    const improvement = ((speedup - 1) * 100).toFixed(1);

    console.log(`${idx + 1}. ${before.description}`);
    console.log(`   Before: ${before.timeMs}ms (index: ${before.indexUsed || 'NONE'})`);
    console.log(`   After:  ${after.timeMs}ms (index: ${after.indexUsed || 'NONE'})`);
    console.log(`   Speedup: ${speedup.toFixed(2)}x (${improvement}% faster)`);
    console.log();
  });

  console.log('Summary:');
  console.log(`   Total time before: ${suite.improvement.totalTimeBefore.toFixed(2)}ms`);
  console.log(`   Total time after:  ${suite.improvement.totalTimeAfter.toFixed(2)}ms`);
  console.log(`   Average speedup:   ${suite.improvement.avgSpeedup.toFixed(2)}x`);
  console.log(`   Overall improvement: ${((suite.improvement.avgSpeedup - 1) * 100).toFixed(1)}%`);
  console.log();
}

/**
 * Main test function
 */
function main() {
  console.log('\n🚀 AgentDB Composite Index Performance Test\n');

  // Create test database
  const db = createTestDatabase();

  // Run benchmarks BEFORE migration
  const resultsBefore = runBenchmarks(db);
  displayResults(resultsBefore, '📈 Results BEFORE Composite Indexes');

  // Apply composite index migration
  console.log('\n⚡ Applying composite index migration...\n');

  const migrationPath = path.join(__dirname, '003_composite_indexes.sql');
  const migration = fs.readFileSync(migrationPath, 'utf-8');

  db.exec(migration);
  db.exec('ANALYZE');

  console.log('   ✅ Migration applied\n');

  // Run benchmarks AFTER migration
  const resultsAfter = runBenchmarks(db);
  displayResults(resultsAfter, '📈 Results AFTER Composite Indexes');

  // Compare results
  const totalBefore = resultsBefore.reduce((sum, r) => sum + r.timeMs, 0);
  const totalAfter = resultsAfter.reduce((sum, r) => sum + r.timeMs, 0);
  const avgSpeedup = totalBefore / totalAfter;

  const suite: BenchmarkSuite = {
    before: resultsBefore,
    after: resultsAfter,
    improvement: {
      avgSpeedup,
      totalTimeBefore: totalBefore,
      totalTimeAfter: totalAfter,
    },
  };

  compareResults(suite);

  // Cleanup
  db.close();

  console.log('✅ Test complete!\n');
}

// Run if called directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}

export { createTestDatabase, benchmarkQuery, runBenchmarks };
