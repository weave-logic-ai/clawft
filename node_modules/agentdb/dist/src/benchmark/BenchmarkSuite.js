/**
 * AgentDB Benchmark Suite
 *
 * Comprehensive benchmarking framework for measuring:
 * - Vector insert performance (batch and single)
 * - Vector search latency (p50, p95, p99)
 * - Memory consumption
 * - Concurrent read/write performance
 * - Quantization performance comparison
 *
 * Usage:
 *   import { BenchmarkSuite, runBenchmarks } from './benchmark/BenchmarkSuite';
 *
 *   const suite = new BenchmarkSuite();
 *   const report = await suite.runAll();
 *   console.log(JSON.stringify(report, null, 2));
 *
 * CLI integration:
 *   npx tsx packages/agentdb/src/benchmark/BenchmarkSuite.ts
 */
import { createBackend, detectBackends } from '../backends/factory.js';
import { QuantizedVectorStore, } from '../quantization/index.js';
// ============================================================================
// Benchmark Base Class
// ============================================================================
/**
 * Abstract base class for benchmarks
 */
export class Benchmark {
    /**
     * Generate a random normalized vector
     */
    generateRandomVector(dimension) {
        const vector = new Float32Array(dimension);
        let norm = 0;
        for (let i = 0; i < dimension; i++) {
            vector[i] = Math.random() * 2 - 1;
            norm += vector[i] * vector[i];
        }
        norm = Math.sqrt(norm);
        if (norm > 0) {
            for (let i = 0; i < dimension; i++) {
                vector[i] /= norm;
            }
        }
        return vector;
    }
    /**
     * Generate multiple random vectors
     */
    generateRandomVectors(count, dimension) {
        const vectors = [];
        for (let i = 0; i < count; i++) {
            vectors.push(this.generateRandomVector(dimension));
        }
        return vectors;
    }
    /**
     * Calculate latency statistics from measurements
     */
    calculateLatencyStats(measurements) {
        if (measurements.length === 0) {
            return { p50: 0, p95: 0, p99: 0, mean: 0, max: 0, min: 0 };
        }
        const sorted = [...measurements].sort((a, b) => a - b);
        const n = sorted.length;
        const percentile = (p) => {
            const index = Math.ceil((p / 100) * n) - 1;
            return sorted[Math.max(0, Math.min(index, n - 1))];
        };
        const sum = sorted.reduce((a, b) => a + b, 0);
        return {
            p50: percentile(50),
            p95: percentile(95),
            p99: percentile(99),
            mean: sum / n,
            max: sorted[n - 1],
            min: sorted[0],
        };
    }
    /**
     * Get current memory usage in MB
     */
    getMemoryUsageMB() {
        if (typeof process !== 'undefined' && process.memoryUsage) {
            return process.memoryUsage().heapUsed / (1024 * 1024);
        }
        return 0;
    }
    /**
     * Force garbage collection if available
     */
    forceGC() {
        if (typeof global !== 'undefined' && global.gc) {
            global.gc();
        }
    }
}
// ============================================================================
// Built-in Benchmarks
// ============================================================================
/**
 * Vector Insert Benchmark
 *
 * Tests batch insert performance at different scales:
 * - 1K vectors (warm cache scenarios)
 * - 10K vectors (typical use case)
 * - 100K vectors (stress test)
 */
export class VectorInsertBenchmark extends Benchmark {
    name = 'VectorInsert';
    description = 'Batch insert performance at different scales';
    backend;
    config;
    vectors1K = [];
    vectors10K = [];
    vectors100K = [];
    constructor(config = {}) {
        super();
        this.config = {
            vectorDimension: config.vectorDimension ?? 384,
            warmupIterations: config.warmupIterations ?? 100,
            insertCounts: config.insertCounts ?? [1000, 10000, 100000],
            searchQueries: config.searchQueries ?? 1000,
            searchK: config.searchK ?? 10,
            concurrencyLevels: config.concurrencyLevels ?? [1, 4, 8, 16],
            runMemoryTests: config.runMemoryTests ?? true,
            runQuantizationTests: config.runQuantizationTests ?? true,
        };
    }
    async setup() {
        // Create backend
        this.backend = await createBackend('auto', {
            dimensions: this.config.vectorDimension,
            metric: 'cosine',
            maxElements: 150000,
        });
        // Pre-generate vectors to exclude generation time from measurements
        console.log('  Generating test vectors...');
        if (this.config.insertCounts.includes(1000)) {
            this.vectors1K = this.generateRandomVectors(1000, this.config.vectorDimension);
        }
        if (this.config.insertCounts.includes(10000)) {
            this.vectors10K = this.generateRandomVectors(10000, this.config.vectorDimension);
        }
        if (this.config.insertCounts.includes(100000)) {
            this.vectors100K = this.generateRandomVectors(100000, this.config.vectorDimension);
        }
    }
    async run() {
        const results = [];
        const latencies = [];
        // Test each vector count
        for (const count of this.config.insertCounts) {
            const vectors = count === 1000 ? this.vectors1K :
                count === 10000 ? this.vectors10K :
                    this.vectors100K;
            if (vectors.length === 0)
                continue;
            // Warmup
            for (let i = 0; i < Math.min(this.config.warmupIterations, 10); i++) {
                this.backend.insert(`warmup-${i}`, vectors[i % vectors.length]);
            }
            this.forceGC();
            const memBefore = this.getMemoryUsageMB();
            const start = performance.now();
            // Batch insert with individual timing
            const batchItems = vectors.map((embedding, idx) => ({
                id: `vec-${count}-${idx}`,
                embedding,
            }));
            // Measure individual insert latencies for a sample
            const sampleSize = Math.min(100, count);
            for (let i = 0; i < sampleSize; i++) {
                const insertStart = performance.now();
                this.backend.insert(`latency-${count}-${i}`, vectors[i]);
                latencies.push(performance.now() - insertStart);
            }
            // Batch the rest
            if (count > sampleSize) {
                this.backend.insertBatch(batchItems.slice(sampleSize));
            }
            const duration = performance.now() - start;
            const memAfter = this.getMemoryUsageMB();
            results.push({
                count,
                duration,
                opsPerSec: (count / duration) * 1000,
            });
            console.log(`    ${count.toLocaleString()} vectors: ${duration.toFixed(2)}ms ` +
                `(${((count / duration) * 1000).toFixed(0)} ops/sec, ` +
                `+${(memAfter - memBefore).toFixed(1)}MB)`);
        }
        // Calculate aggregate statistics
        const totalOps = results.reduce((sum, r) => sum + r.count, 0);
        const totalDuration = results.reduce((sum, r) => sum + r.duration, 0);
        const avgOpsPerSec = results.length > 0
            ? results.reduce((sum, r) => sum + r.opsPerSec, 0) / results.length
            : 0;
        return {
            name: this.name,
            opsPerSecond: avgOpsPerSec,
            latencyMs: this.calculateLatencyStats(latencies),
            memoryMB: this.getMemoryUsageMB(),
            duration: totalDuration,
            operations: totalOps,
            metadata: {
                breakdown: results,
                vectorDimension: this.config.vectorDimension,
            },
        };
    }
    async teardown() {
        this.backend?.close();
        this.vectors1K = [];
        this.vectors10K = [];
        this.vectors100K = [];
        this.forceGC();
    }
}
/**
 * Vector Search Benchmark
 *
 * Tests k-NN search latency with percentile measurements
 */
export class VectorSearchBenchmark extends Benchmark {
    name = 'VectorSearch';
    description = 'k-NN search latency (p50, p95, p99)';
    backend;
    config;
    queryVectors = [];
    constructor(config = {}) {
        super();
        this.config = {
            vectorDimension: config.vectorDimension ?? 384,
            warmupIterations: config.warmupIterations ?? 100,
            insertCounts: config.insertCounts ?? [1000, 10000, 100000],
            searchQueries: config.searchQueries ?? 1000,
            searchK: config.searchK ?? 10,
            concurrencyLevels: config.concurrencyLevels ?? [1, 4, 8, 16],
            runMemoryTests: config.runMemoryTests ?? true,
            runQuantizationTests: config.runQuantizationTests ?? true,
        };
    }
    async setup() {
        this.backend = await createBackend('auto', {
            dimensions: this.config.vectorDimension,
            metric: 'cosine',
            maxElements: 50000,
        });
        // Build index with 10K vectors
        console.log('  Building search index with 10K vectors...');
        const indexVectors = this.generateRandomVectors(10000, this.config.vectorDimension);
        this.backend.insertBatch(indexVectors.map((embedding, idx) => ({
            id: `idx-${idx}`,
            embedding,
        })));
        // Generate query vectors
        this.queryVectors = this.generateRandomVectors(this.config.searchQueries, this.config.vectorDimension);
    }
    async run() {
        const latencies = [];
        // Warmup
        for (let i = 0; i < this.config.warmupIterations; i++) {
            this.backend.search(this.queryVectors[i % this.queryVectors.length], this.config.searchK);
        }
        this.forceGC();
        const memBefore = this.getMemoryUsageMB();
        const start = performance.now();
        // Run search queries and measure latency
        for (const query of this.queryVectors) {
            const queryStart = performance.now();
            this.backend.search(query, this.config.searchK);
            latencies.push(performance.now() - queryStart);
        }
        const duration = performance.now() - start;
        const memAfter = this.getMemoryUsageMB();
        const stats = this.calculateLatencyStats(latencies);
        console.log(`    ${this.queryVectors.length.toLocaleString()} queries: ` +
            `p50=${stats.p50.toFixed(3)}ms, p95=${stats.p95.toFixed(3)}ms, p99=${stats.p99.toFixed(3)}ms`);
        return {
            name: this.name,
            opsPerSecond: (this.queryVectors.length / duration) * 1000,
            latencyMs: stats,
            memoryMB: memAfter - memBefore,
            duration,
            operations: this.queryVectors.length,
            metadata: {
                indexSize: 10000,
                k: this.config.searchK,
                vectorDimension: this.config.vectorDimension,
            },
        };
    }
    async teardown() {
        this.backend?.close();
        this.queryVectors = [];
        this.forceGC();
    }
}
/**
 * Memory Usage Benchmark
 *
 * Tracks memory consumption at different scales
 */
export class MemoryUsageBenchmark extends Benchmark {
    name = 'MemoryUsage';
    description = 'Memory consumption tracking';
    backend;
    config;
    constructor(config = {}) {
        super();
        this.config = {
            vectorDimension: config.vectorDimension ?? 384,
            warmupIterations: config.warmupIterations ?? 100,
            insertCounts: config.insertCounts ?? [1000, 10000, 100000],
            searchQueries: config.searchQueries ?? 1000,
            searchK: config.searchK ?? 10,
            concurrencyLevels: config.concurrencyLevels ?? [1, 4, 8, 16],
            runMemoryTests: config.runMemoryTests ?? true,
            runQuantizationTests: config.runQuantizationTests ?? true,
        };
    }
    async setup() {
        this.forceGC();
    }
    async run() {
        const memoryReadings = [];
        const latencies = [];
        // Test memory at different scales
        const testCounts = [1000, 5000, 10000, 25000, 50000];
        for (const count of testCounts) {
            this.forceGC();
            const memBefore = this.getMemoryUsageMB();
            // Create fresh backend for each test
            const backend = await createBackend('auto', {
                dimensions: this.config.vectorDimension,
                metric: 'cosine',
                maxElements: count + 1000,
            });
            const vectors = this.generateRandomVectors(count, this.config.vectorDimension);
            const start = performance.now();
            backend.insertBatch(vectors.map((embedding, idx) => ({
                id: `mem-${count}-${idx}`,
                embedding,
            })));
            latencies.push(performance.now() - start);
            this.forceGC();
            const memAfter = this.getMemoryUsageMB();
            const memUsed = memAfter - memBefore;
            const bytesPerVector = (memUsed * 1024 * 1024) / count;
            memoryReadings.push({
                vectors: count,
                memoryMB: memUsed,
                bytesPerVector,
            });
            console.log(`    ${count.toLocaleString()} vectors: ${memUsed.toFixed(2)}MB ` +
                `(${bytesPerVector.toFixed(0)} bytes/vector)`);
            backend.close();
        }
        const peakMemory = Math.max(...memoryReadings.map(r => r.memoryMB));
        const avgBytesPerVector = memoryReadings.reduce((sum, r) => sum + r.bytesPerVector, 0) /
            memoryReadings.length;
        return {
            name: this.name,
            opsPerSecond: 0, // Not applicable for memory test
            latencyMs: this.calculateLatencyStats(latencies),
            memoryMB: peakMemory,
            duration: latencies.reduce((a, b) => a + b, 0),
            operations: testCounts.reduce((a, b) => a + b, 0),
            metadata: {
                readings: memoryReadings,
                avgBytesPerVector,
                theoreticalBytesPerVector: this.config.vectorDimension * 4, // Float32
                vectorDimension: this.config.vectorDimension,
            },
        };
    }
    async teardown() {
        this.forceGC();
    }
}
/**
 * Concurrency Benchmark
 *
 * Tests concurrent read/write performance
 */
export class ConcurrencyBenchmark extends Benchmark {
    name = 'Concurrency';
    description = 'Concurrent read/write performance';
    backend;
    config;
    constructor(config = {}) {
        super();
        this.config = {
            vectorDimension: config.vectorDimension ?? 384,
            warmupIterations: config.warmupIterations ?? 100,
            insertCounts: config.insertCounts ?? [1000, 10000, 100000],
            searchQueries: config.searchQueries ?? 1000,
            searchK: config.searchK ?? 10,
            concurrencyLevels: config.concurrencyLevels ?? [1, 4, 8, 16],
            runMemoryTests: config.runMemoryTests ?? true,
            runQuantizationTests: config.runQuantizationTests ?? true,
        };
    }
    async setup() {
        this.backend = await createBackend('auto', {
            dimensions: this.config.vectorDimension,
            metric: 'cosine',
            maxElements: 20000,
        });
        // Pre-populate with vectors for read tests
        console.log('  Pre-populating index for concurrency tests...');
        const vectors = this.generateRandomVectors(5000, this.config.vectorDimension);
        this.backend.insertBatch(vectors.map((embedding, idx) => ({
            id: `pre-${idx}`,
            embedding,
        })));
    }
    async run() {
        const results = [];
        const allLatencies = [];
        for (const concurrency of this.config.concurrencyLevels) {
            const opsPerLevel = 500;
            const latencies = [];
            this.forceGC();
            const start = performance.now();
            // Run concurrent operations
            const workers = [];
            for (let w = 0; w < concurrency; w++) {
                workers.push((async () => {
                    const opsPerWorker = Math.floor(opsPerLevel / concurrency);
                    for (let i = 0; i < opsPerWorker; i++) {
                        const opStart = performance.now();
                        // Mix of reads (70%) and writes (30%)
                        if (Math.random() < 0.7) {
                            // Read
                            const query = this.generateRandomVector(this.config.vectorDimension);
                            this.backend.search(query, this.config.searchK);
                        }
                        else {
                            // Write
                            const vec = this.generateRandomVector(this.config.vectorDimension);
                            this.backend.insert(`conc-${w}-${i}`, vec);
                        }
                        latencies.push(performance.now() - opStart);
                    }
                })());
            }
            await Promise.all(workers);
            const duration = performance.now() - start;
            const opsPerSec = (latencies.length / duration) * 1000;
            const avgLatency = latencies.reduce((a, b) => a + b, 0) / latencies.length;
            results.push({
                concurrency,
                opsPerSec,
                avgLatencyMs: avgLatency,
            });
            allLatencies.push(...latencies);
            console.log(`    Concurrency ${concurrency}: ${opsPerSec.toFixed(0)} ops/sec, ` +
                `avg latency ${avgLatency.toFixed(3)}ms`);
        }
        // Find optimal concurrency
        const bestResult = results.reduce((best, r) => (r.opsPerSec > best.opsPerSec ? r : best), results[0]);
        return {
            name: this.name,
            opsPerSecond: bestResult.opsPerSec,
            latencyMs: this.calculateLatencyStats(allLatencies),
            memoryMB: this.getMemoryUsageMB(),
            duration: results.reduce((sum, r) => sum + (r.opsPerSec > 0 ? 500 / (r.opsPerSec / 1000) : 0), 0),
            operations: results.length * 500,
            metadata: {
                breakdown: results,
                optimalConcurrency: bestResult.concurrency,
                readWriteRatio: '70/30',
            },
        };
    }
    async teardown() {
        this.backend?.close();
        this.forceGC();
    }
}
/**
 * Quantization Benchmark
 *
 * Compares quantized vs unquantized performance
 */
export class QuantizationBenchmark extends Benchmark {
    name = 'Quantization';
    description = 'Quantized vs unquantized performance comparison';
    config;
    constructor(config = {}) {
        super();
        this.config = {
            vectorDimension: config.vectorDimension ?? 384,
            warmupIterations: config.warmupIterations ?? 100,
            insertCounts: config.insertCounts ?? [1000, 10000, 100000],
            searchQueries: config.searchQueries ?? 1000,
            searchK: config.searchK ?? 10,
            concurrencyLevels: config.concurrencyLevels ?? [1, 4, 8, 16],
            runMemoryTests: config.runMemoryTests ?? true,
            runQuantizationTests: config.runQuantizationTests ?? true,
        };
    }
    async setup() {
        this.forceGC();
    }
    async run() {
        const vectorCount = 10000;
        const vectors = this.generateRandomVectors(vectorCount, this.config.vectorDimension);
        const queries = this.generateRandomVectors(100, this.config.vectorDimension);
        const results = [];
        // Test 8-bit quantization
        console.log('  Testing 8-bit scalar quantization...');
        {
            this.forceGC();
            const memBefore = this.getMemoryUsageMB();
            const store = new QuantizedVectorStore({
                dimension: this.config.vectorDimension,
                quantizationType: 'scalar8bit',
                metric: 'cosine',
            });
            const insertStart = performance.now();
            for (let i = 0; i < vectors.length; i++) {
                store.insert(`q8-${i}`, vectors[i]);
            }
            const insertTime = performance.now() - insertStart;
            const searchStart = performance.now();
            for (const query of queries) {
                store.search(query, 10);
            }
            const searchTime = performance.now() - searchStart;
            this.forceGC();
            const memAfter = this.getMemoryUsageMB();
            results.push({
                type: '8-bit',
                insertTime,
                searchTime,
                memoryMB: memAfter - memBefore,
                compressionRatio: 4,
            });
            console.log(`    8-bit: insert=${insertTime.toFixed(0)}ms, search=${searchTime.toFixed(0)}ms, ` +
                `memory=${(memAfter - memBefore).toFixed(1)}MB (4x compression)`);
        }
        // Test 4-bit quantization
        console.log('  Testing 4-bit scalar quantization...');
        {
            this.forceGC();
            const memBefore = this.getMemoryUsageMB();
            const store = new QuantizedVectorStore({
                dimension: this.config.vectorDimension,
                quantizationType: 'scalar4bit',
                metric: 'cosine',
            });
            const insertStart = performance.now();
            for (let i = 0; i < vectors.length; i++) {
                store.insert(`q4-${i}`, vectors[i]);
            }
            const insertTime = performance.now() - insertStart;
            const searchStart = performance.now();
            for (const query of queries) {
                store.search(query, 10);
            }
            const searchTime = performance.now() - searchStart;
            this.forceGC();
            const memAfter = this.getMemoryUsageMB();
            results.push({
                type: '4-bit',
                insertTime,
                searchTime,
                memoryMB: memAfter - memBefore,
                compressionRatio: 8,
            });
            console.log(`    4-bit: insert=${insertTime.toFixed(0)}ms, search=${searchTime.toFixed(0)}ms, ` +
                `memory=${(memAfter - memBefore).toFixed(1)}MB (8x compression)`);
        }
        // Test unquantized (baseline)
        console.log('  Testing unquantized baseline...');
        {
            this.forceGC();
            const memBefore = this.getMemoryUsageMB();
            const backend = await createBackend('auto', {
                dimensions: this.config.vectorDimension,
                metric: 'cosine',
                maxElements: vectorCount + 1000,
            });
            const insertStart = performance.now();
            backend.insertBatch(vectors.map((embedding, idx) => ({
                id: `full-${idx}`,
                embedding,
            })));
            const insertTime = performance.now() - insertStart;
            const searchStart = performance.now();
            for (const query of queries) {
                backend.search(query, 10);
            }
            const searchTime = performance.now() - searchStart;
            this.forceGC();
            const memAfter = this.getMemoryUsageMB();
            results.push({
                type: 'unquantized',
                insertTime,
                searchTime,
                memoryMB: memAfter - memBefore,
                compressionRatio: 1,
            });
            console.log(`    Unquantized: insert=${insertTime.toFixed(0)}ms, search=${searchTime.toFixed(0)}ms, ` +
                `memory=${(memAfter - memBefore).toFixed(1)}MB (1x compression)`);
            backend.close();
        }
        // Calculate latencies from search times
        const allLatencies = results.map(r => r.searchTime / queries.length);
        return {
            name: this.name,
            opsPerSecond: (vectorCount / results.reduce((sum, r) => sum + r.insertTime, 0)) * 1000,
            latencyMs: this.calculateLatencyStats(allLatencies),
            memoryMB: Math.max(...results.map(r => r.memoryMB)),
            duration: results.reduce((sum, r) => sum + r.insertTime + r.searchTime, 0),
            operations: vectorCount * 3 + queries.length * 3,
            metadata: {
                breakdown: results,
                vectorCount,
                queryCount: queries.length,
                vectorDimension: this.config.vectorDimension,
            },
        };
    }
    async teardown() {
        this.forceGC();
    }
}
// ============================================================================
// Benchmark Suite
// ============================================================================
/**
 * BenchmarkSuite - Orchestrates benchmark execution and reporting
 */
export class BenchmarkSuite {
    benchmarks = new Map();
    config;
    constructor(config = {}) {
        this.config = {
            vectorDimension: config.vectorDimension ?? 384,
            warmupIterations: config.warmupIterations ?? 100,
            insertCounts: config.insertCounts ?? [1000, 10000, 100000],
            searchQueries: config.searchQueries ?? 1000,
            searchK: config.searchK ?? 10,
            concurrencyLevels: config.concurrencyLevels ?? [1, 4, 8, 16],
            runMemoryTests: config.runMemoryTests ?? true,
            runQuantizationTests: config.runQuantizationTests ?? true,
        };
        // Register default benchmarks
        this.register(new VectorInsertBenchmark(this.config));
        this.register(new VectorSearchBenchmark(this.config));
        if (this.config.runMemoryTests) {
            this.register(new MemoryUsageBenchmark(this.config));
        }
        this.register(new ConcurrencyBenchmark(this.config));
        if (this.config.runQuantizationTests) {
            this.register(new QuantizationBenchmark(this.config));
        }
    }
    /**
     * Register a benchmark to the suite
     */
    register(benchmark) {
        this.benchmarks.set(benchmark.name, benchmark);
    }
    /**
     * Unregister a benchmark from the suite
     */
    unregister(name) {
        return this.benchmarks.delete(name);
    }
    /**
     * Get list of registered benchmark names
     */
    listBenchmarks() {
        return Array.from(this.benchmarks.keys());
    }
    /**
     * Run all registered benchmarks
     */
    async runAll() {
        const startTime = performance.now();
        const results = [];
        let peakMemory = 0;
        let passed = 0;
        // Detect backend
        const detection = await detectBackends();
        const backendName = detection.available;
        const isNative = detection.ruvector.native;
        console.log('='.repeat(70));
        console.log('AgentDB Benchmark Suite');
        console.log('='.repeat(70));
        console.log(`Backend: ${backendName}${isNative ? ' (native)' : ' (WASM)'}`);
        console.log(`Dimension: ${this.config.vectorDimension}`);
        console.log('='.repeat(70));
        console.log('');
        for (const [name, benchmark] of this.benchmarks) {
            console.log(`Running: ${name}`);
            console.log(`  ${benchmark.description}`);
            try {
                await benchmark.setup();
                const result = await benchmark.run();
                await benchmark.teardown();
                results.push(result);
                peakMemory = Math.max(peakMemory, result.memoryMB);
                passed++;
                console.log(`  Completed: ${result.opsPerSecond.toFixed(0)} ops/sec\n`);
            }
            catch (error) {
                console.error(`  Error: ${error.message}\n`);
                results.push({
                    name,
                    opsPerSecond: 0,
                    latencyMs: { p50: 0, p95: 0, p99: 0, mean: 0, max: 0, min: 0 },
                    memoryMB: 0,
                    duration: 0,
                    operations: 0,
                    metadata: { error: error.message },
                });
            }
        }
        const totalDuration = performance.now() - startTime;
        console.log('='.repeat(70));
        console.log('Summary');
        console.log('='.repeat(70));
        console.log(`Total Duration: ${(totalDuration / 1000).toFixed(2)}s`);
        console.log(`Benchmarks: ${passed}/${this.benchmarks.size} passed`);
        console.log(`Peak Memory: ${peakMemory.toFixed(1)}MB`);
        console.log('='.repeat(70));
        return {
            timestamp: new Date().toISOString(),
            platform: {
                os: typeof process !== 'undefined' ? process.platform : 'unknown',
                arch: typeof process !== 'undefined' ? process.arch : 'unknown',
                nodeVersion: typeof process !== 'undefined' ? process.version : 'unknown',
            },
            backend: {
                name: backendName,
                isNative,
            },
            config: {
                vectorDimension: this.config.vectorDimension,
                warmupIterations: this.config.warmupIterations,
            },
            results,
            summary: {
                totalDuration,
                benchmarksRun: this.benchmarks.size,
                benchmarksPassed: passed,
                peakMemoryMB: peakMemory,
            },
        };
    }
    /**
     * Run a specific benchmark by name
     */
    async runByName(name) {
        const benchmark = this.benchmarks.get(name);
        if (!benchmark) {
            throw new Error(`Benchmark not found: ${name}`);
        }
        console.log(`Running: ${name}`);
        console.log(`  ${benchmark.description}`);
        await benchmark.setup();
        const result = await benchmark.run();
        await benchmark.teardown();
        console.log(`  Completed: ${result.opsPerSecond.toFixed(0)} ops/sec\n`);
        return result;
    }
    /**
     * Compare two benchmark reports
     */
    compare(baseline, current) {
        const comparisons = [];
        const summary = { improved: 0, regressed: 0, unchanged: 0, new: 0, removed: 0 };
        // Create maps for easier lookup
        const baselineMap = new Map(baseline.results.map(r => [r.name, r]));
        const currentMap = new Map(current.results.map(r => [r.name, r]));
        // Get all unique benchmark names
        const allNames = new Set([...baselineMap.keys(), ...currentMap.keys()]);
        for (const name of allNames) {
            const baseResult = baselineMap.get(name) ?? null;
            const currResult = currentMap.get(name) ?? null;
            let status;
            let opsPerSecondChange = null;
            let p99LatencyChange = null;
            let memoryChange = null;
            if (!baseResult) {
                status = 'new';
                summary.new++;
            }
            else if (!currResult) {
                status = 'removed';
                summary.removed++;
            }
            else {
                // Calculate changes
                opsPerSecondChange = baseResult.opsPerSecond > 0
                    ? ((currResult.opsPerSecond - baseResult.opsPerSecond) / baseResult.opsPerSecond) * 100
                    : null;
                p99LatencyChange = baseResult.latencyMs.p99 > 0
                    ? ((currResult.latencyMs.p99 - baseResult.latencyMs.p99) / baseResult.latencyMs.p99) * 100
                    : null;
                memoryChange = baseResult.memoryMB > 0
                    ? ((currResult.memoryMB - baseResult.memoryMB) / baseResult.memoryMB) * 100
                    : null;
                // Determine status based on changes
                const opsImproved = opsPerSecondChange !== null && opsPerSecondChange > 5;
                const opsRegressed = opsPerSecondChange !== null && opsPerSecondChange < -5;
                const latencyImproved = p99LatencyChange !== null && p99LatencyChange < -5;
                const latencyRegressed = p99LatencyChange !== null && p99LatencyChange > 5;
                if (opsImproved || latencyImproved) {
                    status = 'improved';
                    summary.improved++;
                }
                else if (opsRegressed || latencyRegressed) {
                    status = 'regressed';
                    summary.regressed++;
                }
                else {
                    status = 'unchanged';
                    summary.unchanged++;
                }
            }
            comparisons.push({
                name,
                baseline: baseResult,
                current: currResult,
                opsPerSecondChange,
                p99LatencyChange,
                memoryChange,
                status,
            });
        }
        return {
            baselineTimestamp: baseline.timestamp,
            currentTimestamp: current.timestamp,
            comparisons,
            summary,
        };
    }
}
// ============================================================================
// CLI Integration
// ============================================================================
/**
 * Run benchmarks and return results as JSON
 *
 * @param config - Benchmark configuration
 * @returns Promise<BenchmarkReport>
 */
export async function runBenchmarks(config = {}) {
    const suite = new BenchmarkSuite(config);
    return suite.runAll();
}
/**
 * Run benchmarks with a specific subset
 *
 * @param names - Names of benchmarks to run
 * @param config - Benchmark configuration
 * @returns Promise<BenchmarkResult[]>
 */
export async function runSelectedBenchmarks(names, config = {}) {
    const suite = new BenchmarkSuite(config);
    const results = [];
    for (const name of names) {
        try {
            const result = await suite.runByName(name);
            results.push(result);
        }
        catch (error) {
            console.error(`Failed to run ${name}: ${error.message}`);
        }
    }
    return results;
}
/**
 * Format benchmark report as markdown
 */
export function formatReportAsMarkdown(report) {
    const lines = [
        '# AgentDB Benchmark Report',
        '',
        `**Date:** ${report.timestamp}`,
        `**Platform:** ${report.platform.os} ${report.platform.arch} (Node ${report.platform.nodeVersion})`,
        `**Backend:** ${report.backend.name}${report.backend.isNative ? ' (native)' : ''}`,
        `**Vector Dimension:** ${report.config.vectorDimension}`,
        '',
        '## Results',
        '',
        '| Benchmark | Ops/sec | p50 (ms) | p95 (ms) | p99 (ms) | Memory (MB) |',
        '|-----------|---------|----------|----------|----------|-------------|',
    ];
    for (const result of report.results) {
        lines.push(`| ${result.name} | ${result.opsPerSecond.toFixed(0)} | ` +
            `${result.latencyMs.p50.toFixed(3)} | ${result.latencyMs.p95.toFixed(3)} | ` +
            `${result.latencyMs.p99.toFixed(3)} | ${result.memoryMB.toFixed(1)} |`);
    }
    lines.push('');
    lines.push('## Summary');
    lines.push('');
    lines.push(`- **Total Duration:** ${(report.summary.totalDuration / 1000).toFixed(2)}s`);
    lines.push(`- **Benchmarks Passed:** ${report.summary.benchmarksPassed}/${report.summary.benchmarksRun}`);
    lines.push(`- **Peak Memory:** ${report.summary.peakMemoryMB.toFixed(1)}MB`);
    return lines.join('\n');
}
/**
 * Format comparison report as markdown
 */
export function formatComparisonAsMarkdown(comparison) {
    const lines = [
        '# Benchmark Comparison Report',
        '',
        `**Baseline:** ${comparison.baselineTimestamp}`,
        `**Current:** ${comparison.currentTimestamp}`,
        '',
        '## Changes',
        '',
        '| Benchmark | Status | Ops/sec Change | p99 Latency Change | Memory Change |',
        '|-----------|--------|----------------|-------------------|---------------|',
    ];
    for (const comp of comparison.comparisons) {
        const opsChange = comp.opsPerSecondChange !== null
            ? `${comp.opsPerSecondChange > 0 ? '+' : ''}${comp.opsPerSecondChange.toFixed(1)}%`
            : 'N/A';
        const latChange = comp.p99LatencyChange !== null
            ? `${comp.p99LatencyChange > 0 ? '+' : ''}${comp.p99LatencyChange.toFixed(1)}%`
            : 'N/A';
        const memChange = comp.memoryChange !== null
            ? `${comp.memoryChange > 0 ? '+' : ''}${comp.memoryChange.toFixed(1)}%`
            : 'N/A';
        const statusEmoji = comp.status === 'improved' ? 'improved' :
            comp.status === 'regressed' ? 'regressed' :
                comp.status === 'new' ? 'new' :
                    comp.status === 'removed' ? 'removed' : 'unchanged';
        lines.push(`| ${comp.name} | ${statusEmoji} | ${opsChange} | ${latChange} | ${memChange} |`);
    }
    lines.push('');
    lines.push('## Summary');
    lines.push('');
    lines.push(`- **Improved:** ${comparison.summary.improved}`);
    lines.push(`- **Regressed:** ${comparison.summary.regressed}`);
    lines.push(`- **Unchanged:** ${comparison.summary.unchanged}`);
    lines.push(`- **New:** ${comparison.summary.new}`);
    lines.push(`- **Removed:** ${comparison.summary.removed}`);
    return lines.join('\n');
}
// ============================================================================
// CLI Entry Point
// ============================================================================
// Run if executed directly
if (typeof process !== 'undefined' && process.argv[1]?.includes('BenchmarkSuite')) {
    const args = process.argv.slice(2);
    const jsonOutput = args.includes('--json');
    const markdownOutput = args.includes('--markdown');
    // Parse config from command line
    const config = {};
    const dimArg = args.find(a => a.startsWith('--dim='));
    if (dimArg) {
        config.vectorDimension = parseInt(dimArg.split('=')[1], 10);
    }
    const countArg = args.find(a => a.startsWith('--counts='));
    if (countArg) {
        config.insertCounts = countArg.split('=')[1].split(',').map(n => parseInt(n, 10));
    }
    runBenchmarks(config)
        .then(report => {
        if (jsonOutput) {
            console.log(JSON.stringify(report, null, 2));
        }
        else if (markdownOutput) {
            console.log(formatReportAsMarkdown(report));
        }
    })
        .catch(error => {
        console.error('Benchmark failed:', error);
        process.exit(1);
    });
}
//# sourceMappingURL=BenchmarkSuite.js.map