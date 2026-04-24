#!/usr/bin/env node
/**
 * Benchmark for Hook Tools
 * Tests latency and throughput
 */
interface BenchmarkResult {
    tool: string;
    iterations: number;
    avgLatencyMs: number;
    p50Ms: number;
    p95Ms: number;
    p99Ms: number;
    minMs: number;
    maxMs: number;
    throughputOpsPerSec: number;
}
declare function benchmark(name: string, fn: () => Promise<any>, iterations?: number): Promise<BenchmarkResult>;
declare function runBenchmarks(): Promise<BenchmarkResult[]>;
export { runBenchmarks, benchmark };
//# sourceMappingURL=benchmark.d.ts.map