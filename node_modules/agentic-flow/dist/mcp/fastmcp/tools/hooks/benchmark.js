#!/usr/bin/env node
/**
 * Benchmark for Hook Tools
 * Tests latency and throughput
 */
import { hookPreEditTool } from './pre-edit.js';
import { hookPostEditTool } from './post-edit.js';
import { hookPreCommandTool } from './pre-command.js';
import { hookRouteTool } from './route.js';
import { hookMetricsTool } from './metrics.js';
const mockContext = {
    onProgress: () => { }
};
async function benchmark(name, fn, iterations = 100) {
    const latencies = [];
    // Warmup
    for (let i = 0; i < 5; i++) {
        await fn();
    }
    // Benchmark
    const startTime = Date.now();
    for (let i = 0; i < iterations; i++) {
        const start = performance.now();
        await fn();
        latencies.push(performance.now() - start);
    }
    const totalTime = Date.now() - startTime;
    // Calculate percentiles
    latencies.sort((a, b) => a - b);
    const p50 = latencies[Math.floor(iterations * 0.5)];
    const p95 = latencies[Math.floor(iterations * 0.95)];
    const p99 = latencies[Math.floor(iterations * 0.99)];
    return {
        tool: name,
        iterations,
        avgLatencyMs: latencies.reduce((a, b) => a + b, 0) / iterations,
        p50Ms: p50,
        p95Ms: p95,
        p99Ms: p99,
        minMs: latencies[0],
        maxMs: latencies[latencies.length - 1],
        throughputOpsPerSec: (iterations / totalTime) * 1000
    };
}
async function runBenchmarks() {
    console.log('🚀 Hook Tools Benchmark\n');
    console.log('='.repeat(80));
    const results = [];
    // Pre-edit benchmark
    console.log('\n📝 Benchmarking hook_pre_edit...');
    results.push(await benchmark('hook_pre_edit', async () => {
        await hookPreEditTool.execute({ filePath: 'src/index.ts', task: 'Fix bug' }, mockContext);
    }));
    // Post-edit benchmark
    console.log('📝 Benchmarking hook_post_edit...');
    results.push(await benchmark('hook_post_edit', async () => {
        await hookPostEditTool.execute({ filePath: 'src/test.ts', success: true, agent: 'coder' }, mockContext);
    }));
    // Pre-command benchmark
    console.log('📝 Benchmarking hook_pre_command...');
    results.push(await benchmark('hook_pre_command', async () => {
        await hookPreCommandTool.execute({ command: 'npm test' }, mockContext);
    }));
    // Route benchmark
    console.log('📝 Benchmarking hook_route...');
    results.push(await benchmark('hook_route', async () => {
        await hookRouteTool.execute({ task: 'Implement authentication feature' }, mockContext);
    }));
    // Metrics benchmark
    console.log('📝 Benchmarking hook_metrics...');
    results.push(await benchmark('hook_metrics', async () => {
        await hookMetricsTool.execute({ timeframe: '24h', detailed: false }, mockContext);
    }));
    // Print results
    console.log('\n' + '='.repeat(80));
    console.log('\n📊 Results:\n');
    console.log('| Tool | Avg (ms) | P50 (ms) | P95 (ms) | P99 (ms) | Ops/sec |');
    console.log('|------|----------|----------|----------|----------|---------|');
    for (const r of results) {
        console.log(`| ${r.tool.padEnd(16)} | ${r.avgLatencyMs.toFixed(2).padStart(8)} | ` +
            `${r.p50Ms.toFixed(2).padStart(8)} | ${r.p95Ms.toFixed(2).padStart(8)} | ` +
            `${r.p99Ms.toFixed(2).padStart(8)} | ${r.throughputOpsPerSec.toFixed(0).padStart(7)} |`);
    }
    // Latency targets check
    console.log('\n' + '='.repeat(80));
    console.log('\n✅ Latency Target Check:\n');
    const targets = {
        'hook_pre_edit': 20,
        'hook_post_edit': 50,
        'hook_pre_command': 10,
        'hook_route': 30,
        'hook_metrics': 50
    };
    let allPassed = true;
    for (const r of results) {
        const target = targets[r.tool] || 50;
        const passed = r.p95Ms < target;
        allPassed = allPassed && passed;
        console.log(`  ${passed ? '✅' : '❌'} ${r.tool}: P95=${r.p95Ms.toFixed(2)}ms ` +
            `(target: <${target}ms) - ${passed ? 'PASS' : 'FAIL'}`);
    }
    console.log('\n' + '='.repeat(80));
    console.log(`\n${allPassed ? '🎉 All latency targets met!' : '⚠️  Some targets not met'}`);
    // Return results for programmatic use
    return results;
}
// Run if called directly
runBenchmarks().catch(console.error);
export { runBenchmarks, benchmark };
//# sourceMappingURL=benchmark.js.map