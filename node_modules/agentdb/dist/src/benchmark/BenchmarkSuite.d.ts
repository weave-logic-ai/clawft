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
/**
 * Latency statistics with percentiles
 */
export interface LatencyStats {
    /** 50th percentile (median) in milliseconds */
    p50: number;
    /** 95th percentile in milliseconds */
    p95: number;
    /** 99th percentile in milliseconds */
    p99: number;
    /** Mean latency in milliseconds */
    mean: number;
    /** Maximum latency in milliseconds */
    max: number;
    /** Minimum latency in milliseconds */
    min: number;
}
/**
 * Result from a single benchmark run
 */
export interface BenchmarkResult {
    /** Benchmark name */
    name: string;
    /** Operations per second */
    opsPerSecond: number;
    /** Latency statistics */
    latencyMs: LatencyStats;
    /** Memory usage in megabytes */
    memoryMB: number;
    /** Total benchmark duration in milliseconds */
    duration: number;
    /** Total operations performed */
    operations: number;
    /** Optional additional metrics */
    metadata?: Record<string, unknown>;
}
/**
 * Complete benchmark report
 */
export interface BenchmarkReport {
    /** Timestamp of the benchmark run */
    timestamp: string;
    /** Platform information */
    platform: {
        os: string;
        arch: string;
        nodeVersion: string;
    };
    /** Backend information */
    backend: {
        name: string;
        isNative?: boolean;
    };
    /** Configuration used for benchmarks */
    config: {
        vectorDimension: number;
        warmupIterations: number;
    };
    /** Individual benchmark results */
    results: BenchmarkResult[];
    /** Summary statistics */
    summary: {
        totalDuration: number;
        benchmarksRun: number;
        benchmarksPassed: number;
        peakMemoryMB: number;
    };
}
/**
 * Comparison between two benchmark runs
 */
export interface ComparisonReport {
    /** Baseline report timestamp */
    baselineTimestamp: string;
    /** Current report timestamp */
    currentTimestamp: string;
    /** Comparison for each benchmark */
    comparisons: Array<{
        name: string;
        baseline: BenchmarkResult | null;
        current: BenchmarkResult | null;
        /** Percentage change in ops/sec (positive = improvement) */
        opsPerSecondChange: number | null;
        /** Percentage change in p99 latency (negative = improvement) */
        p99LatencyChange: number | null;
        /** Percentage change in memory (negative = improvement) */
        memoryChange: number | null;
        /** Overall assessment */
        status: 'improved' | 'regressed' | 'unchanged' | 'new' | 'removed';
    }>;
    /** Summary of changes */
    summary: {
        improved: number;
        regressed: number;
        unchanged: number;
        new: number;
        removed: number;
    };
}
/**
 * Benchmark configuration options
 */
export interface BenchmarkConfig {
    /** Vector dimension for tests (default: 384) */
    vectorDimension?: number;
    /** Number of warmup iterations before measuring (default: 100) */
    warmupIterations?: number;
    /** Vector counts for insert benchmarks (default: [1000, 10000, 100000]) */
    insertCounts?: number[];
    /** Number of search queries to measure (default: 1000) */
    searchQueries?: number;
    /** k value for k-NN search (default: 10) */
    searchK?: number;
    /** Concurrency levels to test (default: [1, 4, 8, 16]) */
    concurrencyLevels?: number[];
    /** Whether to run memory-intensive tests (default: true) */
    runMemoryTests?: boolean;
    /** Whether to run quantization tests (default: true) */
    runQuantizationTests?: boolean;
}
/**
 * Abstract base class for benchmarks
 */
export declare abstract class Benchmark {
    /** Unique benchmark name */
    abstract readonly name: string;
    /** Description of what this benchmark measures */
    abstract readonly description: string;
    /**
     * Setup phase - prepare resources before benchmarking
     */
    abstract setup(): Promise<void>;
    /**
     * Run the benchmark and return results
     */
    abstract run(): Promise<BenchmarkResult>;
    /**
     * Cleanup phase - release resources after benchmarking
     */
    abstract teardown(): Promise<void>;
    /**
     * Generate a random normalized vector
     */
    protected generateRandomVector(dimension: number): Float32Array;
    /**
     * Generate multiple random vectors
     */
    protected generateRandomVectors(count: number, dimension: number): Float32Array[];
    /**
     * Calculate latency statistics from measurements
     */
    protected calculateLatencyStats(measurements: number[]): LatencyStats;
    /**
     * Get current memory usage in MB
     */
    protected getMemoryUsageMB(): number;
    /**
     * Force garbage collection if available
     */
    protected forceGC(): void;
}
/**
 * Vector Insert Benchmark
 *
 * Tests batch insert performance at different scales:
 * - 1K vectors (warm cache scenarios)
 * - 10K vectors (typical use case)
 * - 100K vectors (stress test)
 */
export declare class VectorInsertBenchmark extends Benchmark {
    readonly name = "VectorInsert";
    readonly description = "Batch insert performance at different scales";
    private backend;
    private config;
    private vectors1K;
    private vectors10K;
    private vectors100K;
    constructor(config?: BenchmarkConfig);
    setup(): Promise<void>;
    run(): Promise<BenchmarkResult>;
    teardown(): Promise<void>;
}
/**
 * Vector Search Benchmark
 *
 * Tests k-NN search latency with percentile measurements
 */
export declare class VectorSearchBenchmark extends Benchmark {
    readonly name = "VectorSearch";
    readonly description = "k-NN search latency (p50, p95, p99)";
    private backend;
    private config;
    private queryVectors;
    constructor(config?: BenchmarkConfig);
    setup(): Promise<void>;
    run(): Promise<BenchmarkResult>;
    teardown(): Promise<void>;
}
/**
 * Memory Usage Benchmark
 *
 * Tracks memory consumption at different scales
 */
export declare class MemoryUsageBenchmark extends Benchmark {
    readonly name = "MemoryUsage";
    readonly description = "Memory consumption tracking";
    private backend;
    private config;
    constructor(config?: BenchmarkConfig);
    setup(): Promise<void>;
    run(): Promise<BenchmarkResult>;
    teardown(): Promise<void>;
}
/**
 * Concurrency Benchmark
 *
 * Tests concurrent read/write performance
 */
export declare class ConcurrencyBenchmark extends Benchmark {
    readonly name = "Concurrency";
    readonly description = "Concurrent read/write performance";
    private backend;
    private config;
    constructor(config?: BenchmarkConfig);
    setup(): Promise<void>;
    run(): Promise<BenchmarkResult>;
    teardown(): Promise<void>;
}
/**
 * Quantization Benchmark
 *
 * Compares quantized vs unquantized performance
 */
export declare class QuantizationBenchmark extends Benchmark {
    readonly name = "Quantization";
    readonly description = "Quantized vs unquantized performance comparison";
    private config;
    constructor(config?: BenchmarkConfig);
    setup(): Promise<void>;
    run(): Promise<BenchmarkResult>;
    teardown(): Promise<void>;
}
/**
 * BenchmarkSuite - Orchestrates benchmark execution and reporting
 */
export declare class BenchmarkSuite {
    private benchmarks;
    private config;
    constructor(config?: BenchmarkConfig);
    /**
     * Register a benchmark to the suite
     */
    register(benchmark: Benchmark): void;
    /**
     * Unregister a benchmark from the suite
     */
    unregister(name: string): boolean;
    /**
     * Get list of registered benchmark names
     */
    listBenchmarks(): string[];
    /**
     * Run all registered benchmarks
     */
    runAll(): Promise<BenchmarkReport>;
    /**
     * Run a specific benchmark by name
     */
    runByName(name: string): Promise<BenchmarkResult>;
    /**
     * Compare two benchmark reports
     */
    compare(baseline: BenchmarkReport, current: BenchmarkReport): ComparisonReport;
}
/**
 * Run benchmarks and return results as JSON
 *
 * @param config - Benchmark configuration
 * @returns Promise<BenchmarkReport>
 */
export declare function runBenchmarks(config?: BenchmarkConfig): Promise<BenchmarkReport>;
/**
 * Run benchmarks with a specific subset
 *
 * @param names - Names of benchmarks to run
 * @param config - Benchmark configuration
 * @returns Promise<BenchmarkResult[]>
 */
export declare function runSelectedBenchmarks(names: string[], config?: BenchmarkConfig): Promise<BenchmarkResult[]>;
/**
 * Format benchmark report as markdown
 */
export declare function formatReportAsMarkdown(report: BenchmarkReport): string;
/**
 * Format comparison report as markdown
 */
export declare function formatComparisonAsMarkdown(comparison: ComparisonReport): string;
//# sourceMappingURL=BenchmarkSuite.d.ts.map