/**
 * WASM Vector Search - Usage Examples
 *
 * Demonstrates how to use WASM-accelerated vector operations in AgentDB
 */

// @ts-ignore - Optional dependency
import Database from 'better-sqlite3';
import { WASMVectorSearch } from '../controllers/WASMVectorSearch.js';
import { EnhancedEmbeddingService } from '../controllers/EnhancedEmbeddingService.js';

// Example 1: Basic Vector Similarity
async function basicSimilarity() {
  console.log('=== Example 1: Basic Vector Similarity ===\n');

  const db = new Database(':memory:');
  const wasmSearch = new WASMVectorSearch(db);

  const vectorA = new Float32Array([1, 0, 0]);
  const vectorB = new Float32Array([0.7, 0.7, 0]);

  const similarity = wasmSearch.cosineSimilarity(vectorA, vectorB);
  console.log(`Cosine similarity: ${similarity.toFixed(4)}\n`);

  db.close();
}

// Example 2: Batch Vector Processing
async function batchProcessing() {
  console.log('=== Example 2: Batch Vector Processing ===\n');

  const db = new Database(':memory:');
  const wasmSearch = new WASMVectorSearch(db);

  const query = new Float32Array(384).fill(0.5);
  const vectors: Float32Array[] = [];

  // Generate 1000 random vectors
  for (let i = 0; i < 1000; i++) {
    const vec = new Float32Array(384);
    for (let j = 0; j < 384; j++) {
      vec[j] = Math.random();
    }
    vectors.push(vec);
  }

  const startTime = performance.now();
  const similarities = wasmSearch.batchSimilarity(query, vectors);
  const duration = performance.now() - startTime;

  console.log(`Processed ${vectors.length} vectors in ${duration.toFixed(2)}ms`);
  console.log(`Throughput: ${((vectors.length / duration) * 1000).toFixed(0)} vectors/sec`);
  console.log(`Average similarity: ${(similarities.reduce((a, b) => a + b) / similarities.length).toFixed(4)}\n`);

  db.close();
}

// Example 3: Build and Search Index
async function indexSearch() {
  console.log('=== Example 3: Index Building and Search ===\n');

  const db = new Database(':memory:');
  const wasmSearch = new WASMVectorSearch(db, { indexThreshold: 500 });

  const vectors: Float32Array[] = [];
  const ids: number[] = [];
  const dimensions = 128;

  // Generate 1500 vectors
  for (let i = 0; i < 1500; i++) {
    const vec = new Float32Array(dimensions);
    for (let j = 0; j < dimensions; j++) {
      vec[j] = Math.random();
    }
    vectors.push(vec);
    ids.push(i);
  }

  console.log(`Building index for ${vectors.length} vectors...`);
  const buildStart = performance.now();
  wasmSearch.buildIndex(vectors, ids);
  const buildTime = performance.now() - buildStart;
  console.log(`Index built in ${buildTime.toFixed(2)}ms\n`);

  // Search the index
  const query = new Float32Array(dimensions).fill(0.5);
  const searchStart = performance.now();
  const results = wasmSearch.searchIndex(query, 10, 0.5);
  const searchTime = performance.now() - searchStart;

  console.log(`Search completed in ${searchTime.toFixed(2)}ms`);
  console.log(`Found ${results.length} results:\n`);

  results.slice(0, 5).forEach((result, idx) => {
    console.log(`  ${idx + 1}. ID: ${result.id}, Similarity: ${result.similarity.toFixed(4)}`);
  });

  const stats = wasmSearch.getStats();
  console.log(`\nIndex stats:`);
  console.log(`  WASM: ${stats.wasmAvailable}`);
  console.log(`  SIMD: ${stats.simdAvailable}`);
  console.log(`  Index size: ${stats.indexSize} vectors\n`);

  db.close();
}

// Example 4: Enhanced Embedding Service
async function enhancedEmbeddings() {
  console.log('=== Example 4: Enhanced Embedding Service ===\n');

  const embedder = new EnhancedEmbeddingService({
    model: 'mock-model',
    dimension: 384,
    provider: 'local',
    enableWASM: true,
    enableBatchProcessing: true,
    batchSize: 50,
  });

  await embedder.initialize();

  // Batch embedding
  const texts = [
    'machine learning',
    'artificial intelligence',
    'deep learning',
    'neural networks',
    'natural language processing',
  ];

  console.log('Batch embedding texts...');
  const startTime = performance.now();
  const embeddings = await embedder.embedBatch(texts);
  const duration = performance.now() - startTime;

  console.log(`Embedded ${texts.length} texts in ${duration.toFixed(2)}ms`);
  console.log(`Average time per text: ${(duration / texts.length).toFixed(2)}ms\n`);

  // Text similarity
  const similarity = await embedder.similarity('machine learning', 'deep learning');
  console.log(`Similarity between "machine learning" and "deep learning": ${similarity.toFixed(4)}\n`);

  // Semantic search
  const corpus = [
    'machine learning algorithms',
    'artificial intelligence research',
    'cooking pasta recipe',
    'neural network training',
    'travel destinations',
    'deep learning frameworks',
  ];

  console.log('Finding most similar texts to "AI and ML"...');
  const results = await embedder.findMostSimilar('AI and ML', corpus, 3);

  results.forEach((result, idx) => {
    console.log(`  ${idx + 1}. "${result.text}" - Similarity: ${result.similarity.toFixed(4)}`);
  });

  const stats = embedder.getStats();
  console.log(`\nService stats:`);
  console.log(`  Cache size: ${stats.cacheSize}`);
  console.log(`  WASM enabled: ${stats.wasmEnabled}`);
  console.log(`  SIMD enabled: ${stats.simdEnabled}\n`);
}

// Example 5: Realistic Use Case - Document Search
async function documentSearch() {
  console.log('=== Example 5: Realistic Document Search ===\n');

  const documents = [
    { id: 1, title: 'Introduction to Machine Learning', content: 'Machine learning is a subset of artificial intelligence...' },
    { id: 2, title: 'Deep Learning Fundamentals', content: 'Deep learning uses neural networks with multiple layers...' },
    { id: 3, title: 'Natural Language Processing', content: 'NLP enables computers to understand human language...' },
    { id: 4, title: 'Computer Vision Basics', content: 'Computer vision allows machines to interpret visual data...' },
    { id: 5, title: 'Italian Cuisine Recipes', content: 'Traditional Italian cooking techniques and pasta recipes...' },
  ];

  const embedder = new EnhancedEmbeddingService({
    model: 'mock-model',
    dimension: 384,
    provider: 'local',
    enableWASM: true,
  });

  await embedder.initialize();

  // Embed all documents
  console.log('Indexing documents...');
  const docTexts = documents.map(d => `${d.title}. ${d.content}`);
  const docEmbeddings = await embedder.embedBatch(docTexts);

  // Search
  const query = 'What are neural networks?';
  console.log(`\nSearching for: "${query}"\n`);

  const queryEmbedding = await embedder.embed(query);

  const db = new Database(':memory:');
  const wasmSearch = new WASMVectorSearch(db, { indexThreshold: 3 });

  wasmSearch.buildIndex(docEmbeddings, documents.map(d => d.id));
  const results = wasmSearch.searchIndex(queryEmbedding, 3);

  console.log('Search results:');
  results.forEach((result, idx) => {
    const doc = documents.find(d => d.id === result.id)!;
    console.log(`  ${idx + 1}. ${doc.title}`);
    console.log(`     Relevance: ${(result.similarity * 100).toFixed(1)}%`);
    console.log(`     Content: ${doc.content.substring(0, 60)}...`);
    console.log();
  });

  db.close();
}

// Run all examples
async function main() {
  console.log('\n');
  console.log('╔════════════════════════════════════════════════════════════════╗');
  console.log('║     WASM Vector Search - Usage Examples                       ║');
  console.log('╚════════════════════════════════════════════════════════════════╝');
  console.log('\n');

  await basicSimilarity();
  await batchProcessing();
  await indexSearch();
  await enhancedEmbeddings();
  await documentSearch();

  console.log('✅ All examples completed successfully!\n');
}

// Execute if run directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(console.error);
}

export {
  basicSimilarity,
  batchProcessing,
  indexSearch,
  enhancedEmbeddings,
  documentSearch,
};
