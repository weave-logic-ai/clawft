#!/usr/bin/env node
/**
 * RuvLLM CLI - Self-learning LLM orchestration
 *
 * Usage:
 *   ruvllm query "What is machine learning?"
 *   ruvllm generate "Write a haiku about AI"
 *   ruvllm memory add "Important context"
 *   ruvllm memory search "context"
 *   ruvllm stats
 *   ruvllm benchmark
 */

const { RuvLLM, SimdOps, version, hasSimdSupport } = require('../dist/cjs/index.js');

const args = process.argv.slice(2);
const command = args[0];

// Parse CLI arguments
function parseArgs(args) {
  const result = { flags: {}, positional: [] };
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg.startsWith('--')) {
      const key = arg.slice(2);
      const nextArg = args[i + 1];
      if (nextArg && !nextArg.startsWith('--')) {
        result.flags[key] = nextArg;
        i++;
      } else {
        result.flags[key] = true;
      }
    } else if (!result.command) {
      result.command = arg;
    } else {
      result.positional.push(arg);
    }
  }
  return result;
}

// Format output
function formatJson(obj) {
  return JSON.stringify(obj, null, 2);
}

function formatTable(data) {
  const maxKeyLen = Math.max(...Object.keys(data).map(k => k.length));
  return Object.entries(data)
    .map(([k, v]) => `  ${k.padEnd(maxKeyLen)} : ${v}`)
    .join('\n');
}

// Commands
async function runQuery(llm, text, flags) {
  const config = {};
  if (flags.temperature) config.temperature = parseFloat(flags.temperature);
  if (flags['max-tokens']) config.maxTokens = parseInt(flags['max-tokens']);
  if (flags['top-p']) config.topP = parseFloat(flags['top-p']);
  if (flags['top-k']) config.topK = parseInt(flags['top-k']);

  const response = llm.query(text, config);

  if (flags.json) {
    console.log(formatJson(response));
  } else {
    console.log('\n' + response.text);
    console.log(`\n--- Model: ${response.model} | Confidence: ${(response.confidence * 100).toFixed(1)}% | Latency: ${response.latencyMs.toFixed(2)}ms ---`);
  }
}

async function runGenerate(llm, prompt, flags) {
  const config = {};
  if (flags.temperature) config.temperature = parseFloat(flags.temperature);
  if (flags['max-tokens']) config.maxTokens = parseInt(flags['max-tokens']);
  if (flags['top-p']) config.topP = parseFloat(flags['top-p']);

  const text = llm.generate(prompt, config);
  console.log(text);
}

async function runMemoryAdd(llm, content, flags) {
  const metadata = flags.metadata ? JSON.parse(flags.metadata) : undefined;
  const id = llm.addMemory(content, metadata);
  console.log(`Added memory with ID: ${id}`);
}

async function runMemorySearch(llm, query, flags) {
  const k = flags.k ? parseInt(flags.k) : 10;
  const results = llm.searchMemory(query, k);

  if (flags.json) {
    console.log(formatJson(results));
  } else {
    if (results.length === 0) {
      console.log('No results found.');
      return;
    }
    results.forEach((r, i) => {
      console.log(`\n[${i + 1}] Score: ${r.score.toFixed(4)} | ID: ${r.id}`);
      console.log(`    ${r.content.slice(0, 100)}${r.content.length > 100 ? '...' : ''}`);
    });
  }
}

async function runStats(llm, flags) {
  const stats = llm.stats();

  if (flags.json) {
    console.log(formatJson(stats));
  } else {
    console.log('\nRuvLLM Statistics:');
    console.log(formatTable({
      'Total Queries': stats.totalQueries,
      'Memory Nodes': stats.memoryNodes,
      'Patterns Learned': stats.patternsLearned,
      'Avg Latency': `${stats.avgLatencyMs.toFixed(2)}ms`,
      'Cache Hit Rate': `${(stats.cacheHitRate * 100).toFixed(1)}%`,
      'Router Accuracy': `${(stats.routerAccuracy * 100).toFixed(1)}%`,
    }));
  }
}

async function runRoute(llm, text, flags) {
  const decision = llm.route(text);

  if (flags.json) {
    console.log(formatJson(decision));
  } else {
    console.log('\nRouting Decision:');
    console.log(formatTable({
      'Model': decision.model,
      'Context Size': decision.contextSize,
      'Temperature': decision.temperature.toFixed(2),
      'Top-P': decision.topP.toFixed(2),
      'Confidence': `${(decision.confidence * 100).toFixed(1)}%`,
    }));
  }
}

async function runEmbed(llm, text, flags) {
  const embedding = llm.embed(text);

  if (flags.json) {
    console.log(formatJson({ embedding, dimensions: embedding.length }));
  } else {
    console.log(`Embedding (${embedding.length} dimensions):`);
    console.log(`  First 10: [${embedding.slice(0, 10).map(x => x.toFixed(4)).join(', ')}...]`);
    console.log(`  Norm: ${Math.sqrt(embedding.reduce((s, x) => s + x * x, 0)).toFixed(4)}`);
  }
}

async function runSimilarity(llm, text1, text2, flags) {
  const score = llm.similarity(text1, text2);

  if (flags.json) {
    console.log(formatJson({ text1, text2, similarity: score }));
  } else {
    console.log(`Similarity: ${(score * 100).toFixed(2)}%`);
  }
}

async function runBenchmark(flags) {
  const simd = new SimdOps();
  const dims = flags.dims ? parseInt(flags.dims) : 768;
  const iterations = flags.iterations ? parseInt(flags.iterations) : 1000;

  // Generate test vectors
  const a = Array.from({ length: dims }, () => Math.random());
  const b = Array.from({ length: dims }, () => Math.random());

  console.log(`\nBenchmark: ${dims} dimensions, ${iterations} iterations`);
  console.log(`SIMD: ${simd.isNative() ? 'Native' : 'JavaScript fallback'}`);
  console.log(`Capabilities: ${simd.capabilities().join(', ')}`);
  console.log('');

  // Dot product benchmark
  let start = Date.now();
  for (let i = 0; i < iterations; i++) {
    simd.dotProduct(a, b);
  }
  let elapsed = Date.now() - start;
  console.log(`Dot Product:        ${elapsed}ms (${(iterations / elapsed * 1000).toFixed(0)} ops/sec)`);

  // Cosine similarity benchmark
  start = Date.now();
  for (let i = 0; i < iterations; i++) {
    simd.cosineSimilarity(a, b);
  }
  elapsed = Date.now() - start;
  console.log(`Cosine Similarity:  ${elapsed}ms (${(iterations / elapsed * 1000).toFixed(0)} ops/sec)`);

  // L2 distance benchmark
  start = Date.now();
  for (let i = 0; i < iterations; i++) {
    simd.l2Distance(a, b);
  }
  elapsed = Date.now() - start;
  console.log(`L2 Distance:        ${elapsed}ms (${(iterations / elapsed * 1000).toFixed(0)} ops/sec)`);

  // Softmax benchmark
  start = Date.now();
  for (let i = 0; i < iterations; i++) {
    simd.softmax(a);
  }
  elapsed = Date.now() - start;
  console.log(`Softmax:            ${elapsed}ms (${(iterations / elapsed * 1000).toFixed(0)} ops/sec)`);
}

async function runInfo(flags) {
  const llm = new RuvLLM();

  const info = {
    version: version(),
    native: llm.isNativeLoaded(),
    simd: hasSimdSupport(),
    capabilities: llm.simdCapabilities(),
    platform: process.platform,
    arch: process.arch,
    nodeVersion: process.version,
  };

  if (flags.json) {
    console.log(formatJson(info));
  } else {
    console.log('\nRuvLLM Info:');
    console.log(formatTable({
      'Version': info.version,
      'Native Module': info.native ? 'Loaded' : 'Fallback (JS)',
      'SIMD Support': info.simd ? 'Yes' : 'No',
      'Capabilities': info.capabilities.join(', '),
      'Platform': `${info.platform}-${info.arch}`,
      'Node.js': info.nodeVersion,
    }));
  }
}

function printHelp() {
  console.log(`
RuvLLM - Self-learning LLM Orchestration

Usage: ruvllm <command> [options]

Commands:
  query <text>              Query the LLM with automatic routing
  generate <prompt>         Generate text with SIMD inference
  route <text>              Get routing decision for query
  memory add <content>      Add content to memory
  memory search <query>     Search memory for similar content
  embed <text>              Get embedding for text
  similarity <t1> <t2>      Compute similarity between texts
  stats                     Show engine statistics
  benchmark                 Run SIMD performance benchmark
  info                      Show system information
  help                      Show this help message

Options:
  --json                    Output as JSON
  --temperature <float>     Sampling temperature (0.0-2.0)
  --max-tokens <int>        Maximum tokens to generate
  --top-p <float>           Nucleus sampling (0.0-1.0)
  --top-k <int>             Top-k sampling
  --k <int>                 Number of results for search
  --metadata <json>         Metadata for memory add
  --dims <int>              Dimensions for benchmark (default: 768)
  --iterations <int>        Iterations for benchmark (default: 1000)

Examples:
  ruvllm query "What is machine learning?"
  ruvllm generate "Write a poem about AI" --temperature 0.9
  ruvllm memory add "Important context" --metadata '{"type":"note"}'
  ruvllm memory search "context" --k 5
  ruvllm similarity "hello world" "hi there"
  ruvllm benchmark --dims 1024 --iterations 5000

Learn more: https://github.com/ruvnet/ruvector
`);
}

// Main
async function main() {
  const parsed = parseArgs(args);
  const { command, positional, flags } = parsed;

  if (!command || command === 'help' || flags.help) {
    printHelp();
    return;
  }

  // Create engine for commands that need it
  const llm = new RuvLLM({
    embeddingDim: flags.dim ? parseInt(flags.dim) : 768,
    learningEnabled: flags['no-learning'] ? false : true,
  });

  try {
    switch (command) {
      case 'query':
        if (!positional[0]) {
          console.error('Error: query text required');
          process.exit(1);
        }
        await runQuery(llm, positional[0], flags);
        break;

      case 'generate':
        if (!positional[0]) {
          console.error('Error: prompt required');
          process.exit(1);
        }
        await runGenerate(llm, positional[0], flags);
        break;

      case 'route':
        if (!positional[0]) {
          console.error('Error: text required');
          process.exit(1);
        }
        await runRoute(llm, positional[0], flags);
        break;

      case 'memory':
        const subcommand = positional[0];
        if (subcommand === 'add') {
          if (!positional[1]) {
            console.error('Error: content required');
            process.exit(1);
          }
          await runMemoryAdd(llm, positional[1], flags);
        } else if (subcommand === 'search') {
          if (!positional[1]) {
            console.error('Error: query required');
            process.exit(1);
          }
          await runMemorySearch(llm, positional[1], flags);
        } else {
          console.error('Error: unknown memory subcommand. Use "add" or "search"');
          process.exit(1);
        }
        break;

      case 'embed':
        if (!positional[0]) {
          console.error('Error: text required');
          process.exit(1);
        }
        await runEmbed(llm, positional[0], flags);
        break;

      case 'similarity':
        if (!positional[0] || !positional[1]) {
          console.error('Error: two texts required');
          process.exit(1);
        }
        await runSimilarity(llm, positional[0], positional[1], flags);
        break;

      case 'stats':
        await runStats(llm, flags);
        break;

      case 'benchmark':
        await runBenchmark(flags);
        break;

      case 'info':
        await runInfo(flags);
        break;

      default:
        console.error(`Unknown command: ${command}`);
        console.error('Run "ruvllm help" for usage information.');
        process.exit(1);
    }
  } catch (error) {
    console.error('Error:', error.message);
    if (flags.verbose) {
      console.error(error.stack);
    }
    process.exit(1);
  }
}

main().catch(err => {
  console.error('Fatal error:', err);
  process.exit(1);
});
