/**
 * AgentDB Benchmark Module
 *
 * Comprehensive benchmarking suite for measuring AgentDB performance.
 *
 * Usage:
 *   import { BenchmarkSuite, runBenchmarks } from '@agentdb/benchmark';
 *
 *   // Run all benchmarks
 *   const report = await runBenchmarks();
 *
 *   // Run with custom config
 *   const report = await runBenchmarks({
 *     vectorDimension: 768,
 *     insertCounts: [1000, 5000],
 *   });
 *
 *   // Run specific benchmarks
 *   const suite = new BenchmarkSuite();
 *   const result = await suite.runByName('VectorSearch');
 *
 * CLI:
 *   npx tsx packages/agentdb/src/benchmark/BenchmarkSuite.ts
 *   npx tsx packages/agentdb/src/benchmark/BenchmarkSuite.ts --json
 *   npx tsx packages/agentdb/src/benchmark/BenchmarkSuite.ts --markdown
 *   npx tsx packages/agentdb/src/benchmark/BenchmarkSuite.ts --dim=768 --counts=1000,5000
 */

export {
  // Main Suite
  BenchmarkSuite,

  // Base class for custom benchmarks
  Benchmark,

  // Built-in benchmarks
  VectorInsertBenchmark,
  VectorSearchBenchmark,
  MemoryUsageBenchmark,
  ConcurrencyBenchmark,
  QuantizationBenchmark,

  // CLI integration functions
  runBenchmarks,
  runSelectedBenchmarks,

  // Formatting utilities
  formatReportAsMarkdown,
  formatComparisonAsMarkdown,

  // Types
  type LatencyStats,
  type BenchmarkResult,
  type BenchmarkReport,
  type ComparisonReport,
  type BenchmarkConfig,
} from './BenchmarkSuite.js';
