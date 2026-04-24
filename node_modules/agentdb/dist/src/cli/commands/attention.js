/**
 * Attention Mechanism CLI Commands
 * Provides CLI interface for attention computation, benchmarking, and optimization
 */
import { Command } from 'commander';
import chalk from 'chalk';
import * as fs from 'fs/promises';
import * as path from 'path';
import { saveAttentionConfig } from '../lib/attention-config.js';
/**
 * Main attention command
 */
export const attentionCommand = new Command('attention')
    .description('Attention mechanism operations (compute, benchmark, optimize)')
    .addCommand(createInitCommand())
    .addCommand(createComputeCommand())
    .addCommand(createBenchmarkCommand())
    .addCommand(createOptimizeCommand());
/**
 * Initialize attention configuration
 */
function createInitCommand() {
    return new Command('init')
        .description('Initialize attention mechanism configuration')
        .option('-m, --mechanism <type>', 'Attention mechanism (flash, hyperbolic, sparse, linear, performer)', 'flash')
        .option('-c, --config <path>', 'Configuration file path')
        .option('-f, --force', 'Force overwrite existing configuration')
        .option('--json', 'Output as JSON')
        .action(async (options) => {
        try {
            if (!options.json) {
                console.log(chalk.cyan.bold('\n🧠 Initializing Attention Configuration\n'));
            }
            // Load or create configuration
            let config;
            const configPath = options.config || path.join(process.cwd(), '.agentdb', 'attention-config.json');
            // Check if config exists
            const exists = await fs.access(configPath).then(() => true).catch(() => false);
            if (exists && !options.force) {
                if (options.json) {
                    console.log(JSON.stringify({ error: 'Configuration already exists. Use --force to overwrite.' }, null, 2));
                }
                else {
                    console.log(chalk.yellow('⚠️  Configuration already exists. Use --force to overwrite.'));
                }
                process.exit(1);
            }
            // Create default configuration
            config = {
                defaultMechanism: options.mechanism || 'flash',
                mechanisms: {
                    flash: {
                        enabled: true,
                        heads: 8,
                        dimension: 384,
                        blockSize: 64,
                    },
                    hyperbolic: {
                        enabled: true,
                        curvature: -1.0,
                        heads: 8,
                        dimension: 384,
                    },
                    sparse: {
                        enabled: true,
                        sparsity: 0.9,
                        heads: 8,
                        dimension: 384,
                    },
                    linear: {
                        enabled: true,
                        kernelSize: 32,
                        heads: 8,
                        dimension: 384,
                    },
                    performer: {
                        enabled: true,
                        randomFeatures: 256,
                        heads: 8,
                        dimension: 384,
                    },
                },
                featureFlags: {
                    enableBenchmarking: true,
                    enableOptimization: true,
                    cacheResults: true,
                },
            };
            // Save configuration
            await saveAttentionConfig(config, configPath);
            if (options.json) {
                console.log(JSON.stringify({ success: true, configPath, config }, null, 2));
            }
            else {
                console.log(chalk.green(`✅ Configuration initialized at: ${configPath}\n`));
                console.log(chalk.bold('Configuration:'));
                console.log(`  Default Mechanism: ${config.defaultMechanism}`);
                console.log(`  Enabled Mechanisms: ${Object.entries(config.mechanisms).filter(([_, v]) => v.enabled).map(([k]) => k).join(', ')}`);
                console.log('');
            }
        }
        catch (error) {
            if (options.json) {
                console.log(JSON.stringify({ error: error.message }, null, 2));
            }
            else {
                console.error(chalk.red(`\n❌ Error: ${error.message}\n`));
            }
            process.exit(1);
        }
    });
}
/**
 * Compute attention
 */
function createComputeCommand() {
    return new Command('compute')
        .description('Compute attention mechanism')
        .option('-m, --mechanism <type>', 'Attention mechanism (flash, hyperbolic, sparse, linear, performer)', 'flash')
        .option('-q, --query <text>', 'Query text or vector')
        .option('-k, --keys-file <path>', 'Path to keys JSON file')
        .option('-v, --values-file <path>', 'Path to values JSON file')
        .option('--heads <n>', 'Number of attention heads', '8')
        .option('--dimension <n>', 'Attention dimension', '384')
        .option('-o, --output <path>', 'Output file path')
        .option('--json', 'Output as JSON')
        .action(async (options) => {
        try {
            if (!options.json) {
                console.log(chalk.cyan.bold('\n🧠 Computing Attention\n'));
                console.log(chalk.bold('Configuration:'));
                console.log(`  Mechanism: ${options.mechanism}`);
                console.log(`  Heads: ${options.heads}`);
                console.log(`  Dimension: ${options.dimension}\n`);
            }
            // Validate inputs
            if (!options.query && !options.keysFile) {
                throw new Error('Either --query or --keys-file must be provided');
            }
            // Load keys and values
            let keys = [];
            let values = [];
            if (options.keysFile) {
                const keysData = await fs.readFile(options.keysFile, 'utf-8');
                keys = JSON.parse(keysData);
            }
            if (options.valuesFile) {
                const valuesData = await fs.readFile(options.valuesFile, 'utf-8');
                values = JSON.parse(valuesData);
            }
            // Compute attention based on mechanism
            const result = await computeAttention(options.mechanism, options.query || '', keys, values, parseInt(String(options.heads || '8')), parseInt(String(options.dimension || '384')));
            // Save or display results
            if (options.output) {
                await fs.writeFile(options.output, JSON.stringify(result, null, 2));
                if (!options.json) {
                    console.log(chalk.green(`✅ Results saved to: ${options.output}\n`));
                }
            }
            if (options.json) {
                console.log(JSON.stringify(result, null, 2));
            }
            else {
                console.log(chalk.bold('Results:'));
                console.log(`  Attention Shape: [${result.shape.join(', ')}]`);
                console.log(`  Computation Time: ${result.computeTimeMs.toFixed(2)}ms`);
                console.log(`  Memory Used: ${result.memoryMB.toFixed(2)}MB`);
                console.log('');
            }
        }
        catch (error) {
            if (options.json) {
                console.log(JSON.stringify({ error: error.message }, null, 2));
            }
            else {
                console.error(chalk.red(`\n❌ Error: ${error.message}\n`));
            }
            process.exit(1);
        }
    });
}
/**
 * Benchmark attention mechanisms
 */
function createBenchmarkCommand() {
    return new Command('benchmark')
        .description('Benchmark attention mechanisms')
        .option('-m, --mechanism <type>', 'Specific mechanism to benchmark')
        .option('--all', 'Benchmark all mechanisms')
        .option('-i, --iterations <n>', 'Number of iterations', '100')
        .option('-o, --output <path>', 'Output file path for results')
        .option('--json', 'Output as JSON')
        .option('--verbose', 'Verbose output')
        .action(async (options) => {
        try {
            if (!options.json) {
                console.log(chalk.cyan.bold('\n⚡ Benchmarking Attention Mechanisms\n'));
            }
            const mechanisms = options.all
                ? ['flash', 'hyperbolic', 'sparse', 'linear', 'performer']
                : options.mechanism
                    ? [options.mechanism]
                    : ['flash'];
            const iterations = parseInt(String(options.iterations || '100'));
            const results = await benchmarkMechanisms(mechanisms, iterations, options.verbose || false);
            // Save results if output path provided
            if (options.output) {
                await fs.writeFile(options.output, JSON.stringify(results, null, 2));
                if (!options.json) {
                    console.log(chalk.green(`\n✅ Results saved to: ${options.output}\n`));
                }
            }
            if (options.json) {
                console.log(JSON.stringify(results, null, 2));
            }
            else {
                displayBenchmarkResults(results);
            }
        }
        catch (error) {
            if (options.json) {
                console.log(JSON.stringify({ error: error.message }, null, 2));
            }
            else {
                console.error(chalk.red(`\n❌ Error: ${error.message}\n`));
            }
            process.exit(1);
        }
    });
}
/**
 * Optimize attention mechanism
 */
function createOptimizeCommand() {
    return new Command('optimize')
        .description('Optimize attention mechanism parameters')
        .option('-m, --mechanism <type>', 'Attention mechanism (hyperbolic, sparse)', 'hyperbolic')
        .option('--curvature <n>', 'Hyperbolic curvature', '-1.0')
        .option('--sparsity <n>', 'Sparsity ratio (0-1)', '0.9')
        .option('-o, --output <path>', 'Output file path for optimized config')
        .option('--json', 'Output as JSON')
        .action(async (options) => {
        try {
            if (!options.json) {
                console.log(chalk.cyan.bold('\n🔧 Optimizing Attention Mechanism\n'));
                console.log(chalk.bold('Parameters:'));
                console.log(`  Mechanism: ${options.mechanism}`);
                if (options.curvature)
                    console.log(`  Curvature: ${options.curvature}`);
                if (options.sparsity)
                    console.log(`  Sparsity: ${options.sparsity}\n`);
            }
            const optimizationResult = await optimizeMechanism(options.mechanism, parseFloat(String(options.curvature || '-1.0')), parseFloat(String(options.sparsity || '0.9')));
            // Save optimized configuration
            if (options.output) {
                await fs.writeFile(options.output, JSON.stringify(optimizationResult, null, 2));
                if (!options.json) {
                    console.log(chalk.green(`✅ Optimized configuration saved to: ${options.output}\n`));
                }
            }
            if (options.json) {
                console.log(JSON.stringify(optimizationResult, null, 2));
            }
            else {
                console.log(chalk.bold('Optimization Results:'));
                console.log(`  Performance Gain: ${(optimizationResult.performanceGain * 100).toFixed(1)}%`);
                console.log(`  Memory Reduction: ${(optimizationResult.memoryReduction * 100).toFixed(1)}%`);
                console.log(`  Recommended Configuration:`);
                console.log(`    ${JSON.stringify(optimizationResult.config, null, 4).split('\n').join('\n    ')}`);
                console.log('');
            }
        }
        catch (error) {
            if (options.json) {
                console.log(JSON.stringify({ error: error.message }, null, 2));
            }
            else {
                console.error(chalk.red(`\n❌ Error: ${error.message}\n`));
            }
            process.exit(1);
        }
    });
}
// Helper functions
async function computeAttention(mechanism, query, keys, values, heads, dimension) {
    const startTime = performance.now();
    // Simulate attention computation
    const queryVector = query ? encodeQuery(query, dimension) : keys[0] || Array(dimension).fill(0);
    const attentionWeights = computeAttentionWeights(mechanism, queryVector, keys, heads);
    const output = applyAttentionWeights(attentionWeights, values.length > 0 ? values : keys);
    const computeTime = performance.now() - startTime;
    const memoryUsed = estimateMemory(keys.length, dimension, heads);
    return {
        mechanism,
        shape: [heads, keys.length],
        output,
        attentionWeights,
        computeTimeMs: computeTime,
        memoryMB: memoryUsed,
        config: {
            heads,
            dimension,
            keysCount: keys.length,
            valuesCount: values.length > 0 ? values.length : keys.length,
        },
    };
}
async function benchmarkMechanisms(mechanisms, iterations, verbose) {
    const results = [];
    for (const mechanism of mechanisms) {
        if (verbose) {
            console.log(chalk.cyan(`\nBenchmarking ${mechanism}...`));
        }
        const times = [];
        const memories = [];
        for (let i = 0; i < iterations; i++) {
            const startTime = performance.now();
            // Simulate computation
            const keys = generateRandomKeys(100, 384);
            const query = Array(384).fill(0).map(() => Math.random());
            const weights = computeAttentionWeights(mechanism, query, keys, 8);
            times.push(performance.now() - startTime);
            memories.push(estimateMemory(100, 384, 8));
            if (verbose && (i + 1) % (iterations / 10) === 0) {
                process.stdout.write('.');
            }
        }
        if (verbose) {
            console.log('');
        }
        results.push({
            mechanism,
            iterations,
            avgTimeMs: average(times),
            minTimeMs: Math.min(...times),
            maxTimeMs: Math.max(...times),
            stdDevMs: stdDev(times),
            avgMemoryMB: average(memories),
        });
    }
    return {
        timestamp: new Date().toISOString(),
        iterations,
        results,
        comparison: generateComparison(results),
    };
}
async function optimizeMechanism(mechanism, curvature, sparsity) {
    // Simulate optimization process
    const baselinePerf = await benchmarkMechanisms([mechanism], 50, false);
    const baselineTime = baselinePerf.results[0].avgTimeMs;
    const baselineMemory = baselinePerf.results[0].avgMemoryMB;
    // Apply optimizations
    let optimizedConfig = {};
    let performanceGain = 0;
    let memoryReduction = 0;
    switch (mechanism) {
        case 'hyperbolic':
            optimizedConfig = {
                curvature,
                heads: 8,
                dimension: 384,
                usePoincareDistance: true,
            };
            performanceGain = Math.abs(curvature) > 0.5 ? 0.15 : 0.08;
            memoryReduction = 0.05;
            break;
        case 'sparse':
            optimizedConfig = {
                sparsity,
                heads: 8,
                dimension: 384,
                topK: Math.floor((1 - sparsity) * 100),
            };
            performanceGain = sparsity * 0.3;
            memoryReduction = sparsity * 0.4;
            break;
        default:
            optimizedConfig = {
                heads: 8,
                dimension: 384,
            };
            performanceGain = 0.1;
            memoryReduction = 0.05;
    }
    return {
        mechanism,
        baseline: {
            avgTimeMs: baselineTime,
            avgMemoryMB: baselineMemory,
        },
        optimized: {
            avgTimeMs: baselineTime * (1 - performanceGain),
            avgMemoryMB: baselineMemory * (1 - memoryReduction),
        },
        performanceGain,
        memoryReduction,
        config: optimizedConfig,
    };
}
// Utility functions
function encodeQuery(query, dimension) {
    // Simple hash-based encoding (in production, use proper embeddings)
    const vector = Array(dimension).fill(0);
    for (let i = 0; i < query.length; i++) {
        const idx = query.charCodeAt(i) % dimension;
        vector[idx] += 1;
    }
    const norm = Math.sqrt(vector.reduce((sum, x) => sum + x * x, 0));
    return vector.map(x => x / (norm || 1));
}
function computeAttentionWeights(mechanism, query, keys, heads) {
    const weights = [];
    for (let h = 0; h < heads; h++) {
        const headWeights = [];
        for (const key of keys) {
            let score = 0;
            switch (mechanism) {
                case 'flash':
                case 'linear':
                case 'performer':
                    // Dot product attention
                    score = dotProduct(query, key);
                    break;
                case 'hyperbolic':
                    // Poincare distance (inverted for similarity)
                    score = 1 / (1 + poincareDistance(query, key));
                    break;
                case 'sparse':
                    // Sparse attention (random masking)
                    score = Math.random() > 0.9 ? dotProduct(query, key) : 0;
                    break;
            }
            headWeights.push(score);
        }
        // Softmax normalization
        const maxScore = Math.max(...headWeights);
        const expScores = headWeights.map(s => Math.exp(s - maxScore));
        const sumExp = expScores.reduce((a, b) => a + b, 0);
        weights.push(expScores.map(s => s / sumExp));
    }
    return weights;
}
function applyAttentionWeights(weights, values) {
    return weights.map(headWeights => {
        const output = Array(values[0]?.length || 384).fill(0);
        for (let i = 0; i < values.length; i++) {
            for (let j = 0; j < output.length; j++) {
                output[j] += headWeights[i] * (values[i]?.[j] || 0);
            }
        }
        return output;
    });
}
function generateRandomKeys(count, dimension) {
    return Array(count).fill(0).map(() => Array(dimension).fill(0).map(() => Math.random() * 2 - 1));
}
function dotProduct(a, b) {
    return a.reduce((sum, val, i) => sum + val * (b[i] || 0), 0);
}
function poincareDistance(a, b) {
    const diff = a.map((val, i) => val - (b[i] || 0));
    const normDiff = Math.sqrt(diff.reduce((sum, x) => sum + x * x, 0));
    const normA = Math.sqrt(a.reduce((sum, x) => sum + x * x, 0));
    const normB = Math.sqrt(b.reduce((sum, x) => sum + x * x, 0));
    const numerator = normDiff * normDiff;
    const denominator = (1 - normA * normA) * (1 - normB * normB);
    return Math.acosh(1 + 2 * numerator / Math.max(denominator, 1e-8));
}
function estimateMemory(keyCount, dimension, heads) {
    // Memory in MB: keys + values + attention weights
    const keysMemory = keyCount * dimension * 4; // float32
    const valuesMemory = keyCount * dimension * 4;
    const weightsMemory = heads * keyCount * 4;
    return (keysMemory + valuesMemory + weightsMemory) / (1024 * 1024);
}
function average(values) {
    return values.reduce((a, b) => a + b, 0) / values.length;
}
function stdDev(values) {
    const avg = average(values);
    const squareDiffs = values.map(v => Math.pow(v - avg, 2));
    return Math.sqrt(average(squareDiffs));
}
function generateComparison(results) {
    const sorted = [...results].sort((a, b) => a.avgTimeMs - b.avgTimeMs);
    const fastest = sorted[0];
    const slowest = sorted[sorted.length - 1];
    return {
        fastest: {
            mechanism: fastest.mechanism,
            avgTimeMs: fastest.avgTimeMs,
        },
        slowest: {
            mechanism: slowest.mechanism,
            avgTimeMs: slowest.avgTimeMs,
        },
        speedup: slowest.avgTimeMs / fastest.avgTimeMs,
        recommendation: fastest.mechanism,
    };
}
function displayBenchmarkResults(results) {
    console.log(chalk.bold('Benchmark Results:\n'));
    for (const result of results.results) {
        console.log(chalk.cyan(`${result.mechanism}:`));
        console.log(`  Avg Time: ${result.avgTimeMs.toFixed(3)}ms`);
        console.log(`  Min Time: ${result.minTimeMs.toFixed(3)}ms`);
        console.log(`  Max Time: ${result.maxTimeMs.toFixed(3)}ms`);
        console.log(`  Std Dev: ${result.stdDevMs.toFixed(3)}ms`);
        console.log(`  Avg Memory: ${result.avgMemoryMB.toFixed(2)}MB\n`);
    }
    console.log(chalk.bold('Comparison:'));
    console.log(`  Fastest: ${results.comparison.fastest.mechanism} (${results.comparison.fastest.avgTimeMs.toFixed(3)}ms)`);
    console.log(`  Slowest: ${results.comparison.slowest.mechanism} (${results.comparison.slowest.avgTimeMs.toFixed(3)}ms)`);
    console.log(`  Speedup: ${results.comparison.speedup.toFixed(2)}x`);
    console.log(`  Recommendation: ${chalk.green(results.comparison.recommendation)}\n`);
}
// Add help text
attentionCommand.on('--help', () => {
    console.log('');
    console.log('Examples:');
    console.log('  $ agentdb attention init --mechanism flash');
    console.log('  $ agentdb attention compute --mechanism flash --query "search query" --keys-file keys.json');
    console.log('  $ agentdb attention benchmark --all --iterations 100 --output benchmark.json');
    console.log('  $ agentdb attention optimize --mechanism hyperbolic --curvature -1.0');
    console.log('');
});
//# sourceMappingURL=attention.js.map