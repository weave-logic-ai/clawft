/**
 * Embedding Benchmark - Compare simple vs ONNX embeddings
 *
 * Run with: npx ts-node src/intelligence/embedding-benchmark.ts
 */
import { getEmbeddingService, EmbeddingService } from './EmbeddingService.js';
const TEST_TEXTS = [
    'Fix a bug in the authentication system',
    'Implement user login functionality',
    'Write unit tests for the API',
    'Refactor the database layer',
    'Optimize memory usage',
    'Add dark mode to the UI',
    'Deploy to production',
    'Review pull request',
    'Document the API endpoints',
    'Set up CI/CD pipeline',
];
const SEMANTIC_PAIRS = [
    { a: 'I love dogs', b: 'I adore puppies', expected: 'high' },
    { a: 'Fix authentication bug', b: 'Repair login issue', expected: 'high' },
    { a: 'Write unit tests', b: 'Create test cases', expected: 'high' },
    { a: 'Deploy to production', b: 'The weather is nice', expected: 'low' },
    { a: 'Fix bug', b: 'Add feature', expected: 'medium' },
    { a: 'Machine learning', b: 'Artificial intelligence', expected: 'high' },
    { a: 'Pizza recipe', b: 'Quantum physics', expected: 'low' },
];
async function runBenchmark() {
    console.log('='.repeat(60));
    console.log('Embedding Benchmark: Simple vs ONNX');
    console.log('='.repeat(60));
    // Benchmark Simple Embeddings
    console.log('\n--- Simple Embeddings ---');
    process.env.AGENTIC_FLOW_EMBEDDINGS = 'simple';
    EmbeddingService.reset();
    const simpleService = getEmbeddingService();
    const simpleResults = await benchmarkService(simpleService, 'simple');
    // Benchmark ONNX Embeddings
    console.log('\n--- ONNX Embeddings ---');
    process.env.AGENTIC_FLOW_EMBEDDINGS = 'onnx';
    EmbeddingService.reset();
    const onnxService = getEmbeddingService();
    const onnxResults = await benchmarkService(onnxService, 'onnx');
    // Summary
    console.log('\n' + '='.repeat(60));
    console.log('SUMMARY');
    console.log('='.repeat(60));
    console.log(`
| Metric                | Simple      | ONNX        |
|-----------------------|-------------|-------------|
| Avg Latency (cold)    | ${simpleResults.avgColdLatency.toFixed(2)}ms     | ${onnxResults.avgColdLatency.toFixed(2)}ms    |
| Avg Latency (warm)    | ${simpleResults.avgWarmLatency.toFixed(2)}ms     | ${onnxResults.avgWarmLatency.toFixed(2)}ms    |
| Batch 10 texts        | ${simpleResults.batchLatency.toFixed(2)}ms     | ${onnxResults.batchLatency.toFixed(2)}ms    |
| Dimension             | ${simpleResults.dimension}         | ${onnxResults.dimension}         |
| Semantic Accuracy     | ${simpleResults.semanticAccuracy}%         | ${onnxResults.semanticAccuracy}%        |
`);
    console.log('\nSemantic Similarity Comparison:');
    console.log('-'.repeat(60));
    for (let i = 0; i < SEMANTIC_PAIRS.length; i++) {
        const pair = SEMANTIC_PAIRS[i];
        console.log(`"${pair.a}" vs "${pair.b}"`);
        console.log(`  Expected: ${pair.expected}`);
        console.log(`  Simple:   ${simpleResults.similarities[i].toFixed(3)}`);
        console.log(`  ONNX:     ${onnxResults.similarities[i].toFixed(3)}`);
        console.log();
    }
    // Recommendation
    console.log('='.repeat(60));
    console.log('RECOMMENDATION');
    console.log('='.repeat(60));
    if (onnxResults.semanticAccuracy > simpleResults.semanticAccuracy + 20) {
        console.log(`
ONNX embeddings provide significantly better semantic accuracy
(${onnxResults.semanticAccuracy}% vs ${simpleResults.semanticAccuracy}%).

For tasks requiring semantic understanding (routing, pattern matching),
use ONNX embeddings:

  export AGENTIC_FLOW_EMBEDDINGS=onnx

Note: First embedding takes ~${(onnxResults.avgColdLatency / 1000).toFixed(1)}s (model loading).
Subsequent embeddings: ~${onnxResults.avgWarmLatency.toFixed(1)}ms.
`);
    }
    else {
        console.log(`
Simple embeddings are sufficient for your use case.
Semantic accuracy difference is minimal.

Keep using simple embeddings for maximum speed:

  export AGENTIC_FLOW_EMBEDDINGS=simple
`);
    }
}
async function benchmarkService(service, name) {
    // Cold start (first embedding, includes model loading for ONNX)
    console.log(`\n[${name}] Cold start embedding...`);
    const coldStart = performance.now();
    await service.embed(TEST_TEXTS[0]);
    const coldLatency = performance.now() - coldStart;
    console.log(`  Cold latency: ${coldLatency.toFixed(2)}ms`);
    // Warm embeddings
    console.log(`[${name}] Warm embeddings (${TEST_TEXTS.length} texts)...`);
    service.clearCache();
    const warmStart = performance.now();
    for (const text of TEST_TEXTS) {
        await service.embed(text);
    }
    const warmTotalLatency = performance.now() - warmStart;
    const avgWarmLatency = warmTotalLatency / TEST_TEXTS.length;
    console.log(`  Total: ${warmTotalLatency.toFixed(2)}ms, Avg: ${avgWarmLatency.toFixed(2)}ms`);
    // Batch embedding
    console.log(`[${name}] Batch embedding (10 texts)...`);
    service.clearCache();
    const batchStart = performance.now();
    await service.embedBatch(TEST_TEXTS);
    const batchLatency = performance.now() - batchStart;
    const batchPerText = batchLatency / TEST_TEXTS.length;
    console.log(`  Batch latency: ${batchLatency.toFixed(2)}ms (${batchPerText.toFixed(2)}ms per text)`);
    // Compare batch vs sequential
    const speedup = avgWarmLatency > 0 ? avgWarmLatency / batchPerText : 0;
    console.log(`  Batch speedup: ${speedup.toFixed(1)}x vs sequential`);
    // Semantic similarity tests
    console.log(`[${name}] Semantic similarity tests...`);
    const similarities = [];
    let correctCount = 0;
    for (const pair of SEMANTIC_PAIRS) {
        const sim = await service.similarity(pair.a, pair.b);
        similarities.push(sim);
        // For ONNX (semantic), use proper thresholds
        // For simple (hash-based), it will score incorrectly on unrelated pairs
        const isCorrect = (pair.expected === 'high' && sim > 0.5) ||
            (pair.expected === 'medium' && sim >= 0.2 && sim <= 0.6) ||
            (pair.expected === 'low' && sim < 0.3);
        if (isCorrect)
            correctCount++;
        console.log(`  "${pair.a.substring(0, 20)}..." vs "${pair.b.substring(0, 20)}...": ${sim.toFixed(3)} (expected: ${pair.expected})`);
    }
    const semanticAccuracy = Math.round((correctCount / SEMANTIC_PAIRS.length) * 100);
    console.log(`  Semantic accuracy: ${semanticAccuracy}%`);
    const stats = service.getStats();
    console.log(`  Model: ${stats.modelName || 'N/A'}, SIMD: ${stats.simdAvailable ?? 'N/A'}`);
    return {
        avgColdLatency: coldLatency,
        avgWarmLatency,
        batchLatency,
        dimension: stats.dimension,
        semanticAccuracy,
        similarities,
    };
}
// Run if executed directly
runBenchmark().catch(console.error);
//# sourceMappingURL=embedding-benchmark.js.map