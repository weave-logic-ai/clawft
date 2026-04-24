# @ruvector/ruvllm

**Build AI that learns and improves from every interaction.**

RuvLLM is a self-learning language model toolkit that gets smarter over time. Unlike traditional LLMs that remain static after training, RuvLLM continuously adapts to your use case while remembering what it learned before.

## What Makes RuvLLM Different?

Traditional LLMs forget old knowledge when learning new things (called "catastrophic forgetting"). RuvLLM solves this with three key innovations:

1. **It Learns Without Forgetting** - Uses tiny parameter updates (LoRA) and memory protection (EWC++) to learn new patterns while preserving existing knowledge

2. **It Remembers Context** - Built-in vector memory stores and retrieves relevant information instantly using similarity search

3. **It Routes Intelligently** - Automatically selects the right model size and parameters based on query complexity, saving resources on simple tasks

## Key Features

| Feature | What It Does | Why It Matters |
|---------|-------------|----------------|
| **Adaptive Learning** | Learns from user feedback in real-time | Improves accuracy over time without retraining |
| **Memory System** | Stores context with instant similarity search | Finds relevant information in microseconds |
| **Smart Routing** | Picks optimal model/settings per query | Reduces costs, improves response quality |
| **SIMD Acceleration** | Uses CPU vector instructions (AVX2/NEON) | 10-50x faster vector operations |
| **Federated Learning** | Train across devices without sharing data | Privacy-preserving distributed learning |
| **LoRA Adapters** | Parameter-efficient fine-tuning with low-rank matrices | Fast adaptation with minimal memory |
| **EWC++ Protection** | Elastic Weight Consolidation prevents forgetting | Learn new tasks without losing old knowledge |
| **SafeTensors Export** | HuggingFace-compatible model serialization | Share models with the ML ecosystem |
| **Training Pipeline** | Full training infrastructure with schedulers | Production-ready model training |
| **Session Management** | Stateful conversations with streaming | Build chat applications easily |

## Installation

```bash
npm install @ruvector/ruvllm
```

Or run directly:

```bash
npx @ruvector/ruvllm info
```

## Quick Start Tutorial

### 1. Basic Query

```typescript
import { RuvLLM } from '@ruvector/ruvllm';

const llm = new RuvLLM();

// Ask a question - routing happens automatically
const response = llm.query('Explain neural networks simply');
console.log(response.text);
// Output: "Neural networks are computing systems inspired by..."

console.log(`Used model: ${response.model}`);
console.log(`Confidence: ${(response.confidence * 100).toFixed(1)}%`);
```

### 2. Teaching the System

```typescript
// Query and get a response
const response = llm.query('What is the capital of France?');

// Provide feedback - the system learns from this
llm.feedback({
  requestId: response.requestId,
  rating: 5,  // 1-5 scale
  correction: 'Paris is the capital and largest city of France'
});

// Future similar queries will be more accurate
```

### 3. Using Memory

```typescript
// Store important context
llm.addMemory('Company policy: All returns accepted within 30 days', {
  category: 'policy',
  department: 'customer-service'
});

llm.addMemory('Product X launched in March 2024 with features A, B, C', {
  category: 'product',
  name: 'Product X'
});

// Search memory for relevant context
const results = llm.searchMemory('return policy', 5);
console.log(results[0].content);
// Output: "Company policy: All returns accepted within 30 days"
console.log(`Relevance: ${(results[0].score * 100).toFixed(1)}%`);
```

### 4. Computing Similarity

```typescript
import { SimdOps } from '@ruvector/ruvllm';

const simd = new SimdOps();

// Compare two texts
const score = llm.similarity(
  'How do I reset my password?',
  'I forgot my login credentials'
);
console.log(`Similarity: ${(score * 100).toFixed(1)}%`);
// Output: "Similarity: 78.3%"

// Fast vector operations
const embedding1 = llm.embed('machine learning');
const embedding2 = llm.embed('deep learning');
const similarity = simd.cosineSimilarity(embedding1, embedding2);
```

### 5. Batch Processing

```typescript
// Process multiple queries efficiently
const batch = llm.batchQuery({
  queries: [
    'What is AI?',
    'Explain machine learning',
    'How do neural networks work?'
  ],
  config: { temperature: 0.7 }
});

batch.responses.forEach((r, i) => {
  console.log(`Query ${i + 1}: ${r.text.slice(0, 50)}...`);
});
console.log(`Total time: ${batch.totalLatencyMs}ms`);
```

## CLI Commands

```bash
# Get system information
ruvllm info

# Query the model
ruvllm query "What is quantum computing?"

# Generate text with custom settings
ruvllm generate "Write a product description for:" --temperature 0.8 --max-tokens 200

# Memory operations
ruvllm memory add "Important fact to remember"
ruvllm memory search "fact" --k 10

# Compare texts
ruvllm similarity "hello world" "hi there"

# Get embeddings
ruvllm embed "your text here"

# Run performance benchmark
ruvllm benchmark --dims 768 --iterations 5000

# View statistics
ruvllm stats --json
```

## Benchmarks

*Benchmarked in Docker (node:20-alpine, x64) - December 2024*

### Core Operations

| Operation | Time | Throughput |
|-----------|------|------------|
| Query (short) | 1.49μs | **670K ops/s** |
| Query (long) | 874ns | **1.14M ops/s** |
| Generate | 88ns | **11.4M ops/s** |
| Route | 92ns | **10.9M ops/s** |
| Embed (256d) | 10.6μs | **94K ops/s** |
| Embed (768d) | 7.1μs | **140K ops/s** |

### SIMD Vector Operations

| Operation | 128d | 256d | 512d | 768d |
|-----------|------|------|------|------|
| Dot Product | 214ns / **4.67M ops/s** | 318ns / **3.15M ops/s** | 609ns / **1.64M ops/s** | 908ns / **1.10M ops/s** |
| Cosine Similarity | 233ns / **4.30M ops/s** | 335ns / **2.99M ops/s** | 652ns / **1.53M ops/s** | 972ns / **1.03M ops/s** |
| L2 Distance | 195ns / **5.14M ops/s** | 315ns / **3.18M ops/s** | 612ns / **1.63M ops/s** | 929ns / **1.08M ops/s** |

### LoRA Adapter Performance

| Operation | 64d | 128d | 256d |
|-----------|-----|------|------|
| Forward (r=4) | 6.09μs / **164K ops/s** | 2.74μs / **365K ops/s** | 4.83μs / **207K ops/s** |
| Forward (r=8) | 2.17μs / **462K ops/s** | 4.30μs / **233K ops/s** | 8.99μs / **111K ops/s** |
| Forward (r=16) | 4.85μs / **206K ops/s** | 9.05μs / **111K ops/s** | 18.3μs / **55K ops/s** |
| Backward (r=8) | - | 110μs / **9.1K ops/s** | - |
| Batch (100) | - | 467μs / **2.1K ops/s** | - |

### Memory Operations

| Operation | Time | Throughput |
|-----------|------|------------|
| Add Memory | 5.3μs | **189K ops/s** |
| Search (k=5) | 45.6μs | **21.9K ops/s** |
| Search (k=10) | 28.3μs | **35.3K ops/s** |
| Search (k=20) | 33.1μs | **30.2K ops/s** |

### SONA Learning System

| Operation | Time | Throughput |
|-----------|------|------------|
| Pattern Store | 14.4μs | **69.5K ops/s** |
| Pattern Find Similar | 224μs | **4.5K ops/s** |
| EWC Register Task | 6.5μs | **154K ops/s** |
| EWC Compute Penalty | 501μs | **2.0K ops/s** |
| Trajectory Build | 1.24μs | **807K ops/s** |

### Federated Learning

| Operation | Time | Throughput |
|-----------|------|------------|
| Agent Create | 7.8μs | **128K ops/s** |
| Process Task | 7.9μs | **126K ops/s** |
| Apply LoRA | 12.6μs | **79.6K ops/s** |
| Export State | 48.9μs | **20.4K ops/s** |
| Aggregate | 5.26ms | **190 ops/s** |

### Session & Streaming

| Operation | Time | Throughput |
|-----------|------|------------|
| Session Create | 1.45μs | **690K ops/s** |
| Session Chat | 3.28μs | **305K ops/s** |
| Session Export | 3.91ms | **255 ops/s** |
| Session Import | 1.60ms | **625 ops/s** |

### Training Pipeline

| Operation | Time |
|-----------|------|
| Pipeline Create | 70.6μs |
| Add Data (100 samples) | 70.6μs |
| Train (32 samples, 3 epochs) | 1.33s |

### Export/Import

| Operation | Time | Throughput |
|-----------|------|------------|
| SafeTensors Write | 67.3μs | **14.9K ops/s** |
| SafeTensors Read | 102μs | **9.8K ops/s** |
| LoRA to JSON | 87.9μs | **11.4K ops/s** |
| LoRA from JSON | 86.0μs | **11.6K ops/s** |

### Performance Highlights

- **Fastest**: Generate at **11.4M ops/s**, Route at **10.9M ops/s**
- **Vector Ops**: Up to **5.14M ops/s** for L2 distance (128d)
- **LoRA Forward**: Up to **462K ops/s** (64d, rank-8)
- **Memory Search**: **35K ops/s** (k=10)
- **Session Create**: **690K ops/s**

## Configuration

```typescript
const llm = new RuvLLM({
  // Embedding settings
  embeddingDim: 768,        // Vector dimensions (384, 768, 1024)

  // Memory settings
  hnswM: 16,                // Graph connectivity (higher = better recall, more memory)
  hnswEfConstruction: 100,  // Build quality (higher = better index, slower build)
  hnswEfSearch: 64,         // Search quality (higher = better recall, slower search)

  // Learning settings
  learningEnabled: true,    // Enable adaptive learning
  qualityThreshold: 0.7,    // Min confidence to skip learning
  ewcLambda: 2000,          // Memory protection strength

  // Router settings
  routerHiddenDim: 128,     // Router network size
});
```

## Platform Support

Native acceleration available on:

| Platform | Architecture | SIMD Support |
|----------|-------------|--------------|
| macOS | Apple Silicon (M1/M2/M3) | NEON |
| macOS | Intel x64 | AVX2, SSE4.1 |
| Linux | x64 | AVX2, AVX-512, SSE4.1 |
| Linux | ARM64 | NEON |
| Windows | x64 | AVX2, SSE4.1 |

Falls back to optimized JavaScript on unsupported platforms.

## Real-World Use Cases

### Customer Support Bot
```typescript
// Store FAQ and policies
faqs.forEach(faq => llm.addMemory(faq.answer, { question: faq.question }));

// Answer questions with context
function answerQuestion(question: string) {
  const context = llm.searchMemory(question, 3);
  const prompt = `Context:\n${context.map(c => c.content).join('\n')}\n\nQuestion: ${question}`;
  return llm.query(prompt);
}
```

### Document Search
```typescript
// Index documents
documents.forEach(doc => {
  llm.addMemory(doc.content, {
    title: doc.title,
    path: doc.path
  });
});

// Semantic search
const results = llm.searchMemory('quarterly revenue growth', 10);
```

### Personalized Recommendations
```typescript
// Learn from user interactions
function recordInteraction(userId: string, itemId: string, rating: number) {
  const response = llm.query(`User ${userId} rated ${itemId}`);
  llm.feedback({ requestId: response.requestId, rating });
}

// Get recommendations
function recommend(userId: string) {
  return llm.searchMemory(`preferences for user ${userId}`, 10);
}
```

## API Reference

### RuvLLM Class

| Method | Description |
|--------|-------------|
| `query(text, config?)` | Query with automatic model routing |
| `generate(prompt, config?)` | Generate text with given prompt |
| `route(text)` | Get routing decision without executing |
| `addMemory(content, metadata?)` | Store content in vector memory |
| `searchMemory(text, k?)` | Find similar content (default k=10) |
| `feedback(fb)` | Submit feedback for learning |
| `embed(text)` | Get embedding vector for text |
| `similarity(t1, t2)` | Compute similarity between texts |
| `stats()` | Get engine statistics |
| `forceLearn()` | Trigger immediate learning cycle |

### SimdOps Class

| Method | Description |
|--------|-------------|
| `dotProduct(a, b)` | Vector dot product |
| `cosineSimilarity(a, b)` | Cosine similarity (0-1) |
| `l2Distance(a, b)` | Euclidean distance |
| `normalize(v)` | Normalize to unit length |
| `softmax(v)` | Softmax activation |
| `relu(v)` | ReLU activation |
| `gelu(v)` | GELU activation |
| `layerNorm(v, eps?)` | Layer normalization |
| `matvec(m, v)` | Matrix-vector multiply |

## Troubleshooting

**Q: Native module not loading?**
```bash
ruvllm info  # Check if native is loaded
```
If "Native: Fallback", install platform-specific package manually:
```bash
npm install @ruvector/ruvllm-darwin-arm64  # For Apple Silicon
```

**Q: Memory usage too high?**
Reduce HNSW parameters:
```typescript
const llm = new RuvLLM({ hnswM: 8, hnswEfConstruction: 50 });
```

**Q: Learning not improving results?**
Check that feedback is being processed:
```typescript
const stats = llm.stats();
console.log(`Patterns learned: ${stats.patternsLearned}`);
```

## License

MIT OR Apache-2.0

## Links

- [GitHub Repository](https://github.com/ruvnet/ruvector)
- [Documentation](https://github.com/ruvnet/ruvector/tree/main/examples/ruvLLM)
- [Issue Tracker](https://github.com/ruvnet/ruvector/issues)
