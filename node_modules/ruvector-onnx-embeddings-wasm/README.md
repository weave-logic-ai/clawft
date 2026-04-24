# RuVector ONNX Embeddings WASM

[![npm version](https://img.shields.io/npm/v/ruvector-onnx-embeddings-wasm.svg)](https://www.npmjs.com/package/ruvector-onnx-embeddings-wasm)
[![crates.io](https://img.shields.io/crates/v/ruvector-onnx-embeddings-wasm.svg)](https://crates.io/crates/ruvector-onnx-embeddings-wasm)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![WebAssembly](https://img.shields.io/badge/WebAssembly-654FF0?logo=webassembly&logoColor=white)](https://webassembly.org/)

> **Portable embedding generation that runs anywhere WebAssembly runs**

Generate text embeddings directly in browsers, Cloudflare Workers, Deno, and any WASM runtime. Built with [Tract](https://github.com/sonos/tract) for pure Rust ONNX inference.

## Features

| Feature | Description |
|---------|-------------|
| 🌐 **Browser Support** | Generate embeddings client-side, no server needed |
| ⚡ **Edge Computing** | Deploy to Cloudflare Workers, Vercel Edge, Deno Deploy |
| 📦 **Zero Dependencies** | Single WASM binary, no native modules |
| 🤗 **HuggingFace Models** | Pre-configured URLs for popular models |
| 🔄 **Auto Caching** | Browser Cache API for instant reloads |
| 🎯 **Same API** | Compatible with native `ruvector-onnx-embeddings` |

## Quick Start

### Browser (ES Modules)

```html
<script type="module">
import init, { WasmEmbedder } from 'https://unpkg.com/ruvector-onnx-embeddings-wasm/ruvector_onnx_embeddings_wasm.js';
import { createEmbedder } from 'https://unpkg.com/ruvector-onnx-embeddings-wasm/loader.js';

// Initialize WASM
await init();

// Create embedder (downloads model automatically)
const embedder = await createEmbedder('all-MiniLM-L6-v2');

// Generate embeddings
const embedding = embedder.embedOne("Hello, world!");
console.log("Dimension:", embedding.length); // 384

// Compute similarity
const sim = embedder.similarity("I love Rust", "Rust is great");
console.log("Similarity:", sim.toFixed(4)); // ~0.85
</script>
```

### Node.js

```bash
npm install ruvector-onnx-embeddings-wasm
```

```javascript
import { createEmbedder, similarity, embed } from 'ruvector-onnx-embeddings-wasm/loader.js';

// One-liner similarity
const score = await similarity("I love dogs", "I adore puppies");
console.log(score); // ~0.85

// One-liner embedding
const embedding = await embed("Hello world");
console.log(embedding.length); // 384

// Full control
const embedder = await createEmbedder('bge-small-en-v1.5');
const emb1 = embedder.embedOne("First text");
const emb2 = embedder.embedOne("Second text");
```

### Cloudflare Workers

```javascript
import { WasmEmbedder, WasmEmbedderConfig } from 'ruvector-onnx-embeddings-wasm';

export default {
  async fetch(request, env) {
    // Load model from R2 or KV
    const modelBytes = await env.MODELS.get('model.onnx', 'arrayBuffer');
    const tokenizerJson = await env.MODELS.get('tokenizer.json', 'text');

    const embedder = new WasmEmbedder(
      new Uint8Array(modelBytes),
      tokenizerJson
    );

    const { text } = await request.json();
    const embedding = embedder.embedOne(text);

    return Response.json({
      embedding: Array.from(embedding),
      dimension: embedding.length
    });
  }
};
```

## Available Models

| Model | Dimension | Size | Speed | Quality | Best For |
|-------|-----------|------|-------|---------|----------|
| **all-MiniLM-L6-v2** ⭐ | 384 | 23MB | ⚡⚡⚡ | ⭐⭐⭐ | Default, fast |
| **all-MiniLM-L12-v2** | 384 | 33MB | ⚡⚡ | ⭐⭐⭐⭐ | Better quality |
| **bge-small-en-v1.5** | 384 | 33MB | ⚡⚡⚡ | ⭐⭐⭐⭐ | State-of-the-art |
| **bge-base-en-v1.5** | 768 | 110MB | ⚡ | ⭐⭐⭐⭐⭐ | Best quality |
| **e5-small-v2** | 384 | 33MB | ⚡⚡⚡ | ⭐⭐⭐⭐ | Search/retrieval |
| **gte-small** | 384 | 33MB | ⚡⚡⚡ | ⭐⭐⭐⭐ | Multilingual |

## API Reference

### ModelLoader

```javascript
import { ModelLoader, MODELS, DEFAULT_MODEL } from './loader.js';

// List available models
console.log(ModelLoader.listModels());

// Load with progress
const loader = new ModelLoader({
  cache: true,
  onProgress: ({ percent }) => console.log(`${percent}%`)
});

const { modelBytes, tokenizerJson, config } = await loader.loadModel('all-MiniLM-L6-v2');
```

### WasmEmbedder

```typescript
class WasmEmbedder {
  constructor(modelBytes: Uint8Array, tokenizerJson: string);

  static withConfig(
    modelBytes: Uint8Array,
    tokenizerJson: string,
    config: WasmEmbedderConfig
  ): WasmEmbedder;

  embedOne(text: string): Float32Array;
  embedBatch(texts: string[]): Float32Array;
  similarity(text1: string, text2: string): number;

  dimension(): number;
  maxLength(): number;
}
```

### WasmEmbedderConfig

```typescript
class WasmEmbedderConfig {
  constructor();
  setMaxLength(length: number): WasmEmbedderConfig;
  setNormalize(normalize: boolean): WasmEmbedderConfig;
  setPooling(strategy: number): WasmEmbedderConfig;
  // 0=Mean, 1=Cls, 2=Max, 3=MeanSqrtLen, 4=LastToken
}
```

### Utility Functions

```typescript
function cosineSimilarity(a: Float32Array, b: Float32Array): number;
function normalizeL2(embedding: Float32Array): Float32Array;
function version(): string;
function simd_available(): boolean;
```

## Pooling Strategies

| Value | Strategy | Description |
|-------|----------|-------------|
| 0 | **Mean** | Average all tokens (default, recommended) |
| 1 | **Cls** | Use [CLS] token only (BERT-style) |
| 2 | **Max** | Max pooling across tokens |
| 3 | **MeanSqrtLen** | Mean normalized by sqrt(length) |
| 4 | **LastToken** | Last token (decoder models) |

## Performance

| Environment | Throughput | Latency |
|-------------|------------|---------|
| Chrome (M1 Mac) | ~50 texts/sec | ~20ms |
| Firefox (M1 Mac) | ~45 texts/sec | ~22ms |
| Node.js 20 | ~80 texts/sec | ~12ms |
| Cloudflare Workers | ~30 texts/sec | ~33ms |
| Deno | ~75 texts/sec | ~13ms |

*Tested with all-MiniLM-L6-v2, 128 token inputs*

## Comparison: Native vs WASM

| Aspect | Native (`ort`) | WASM (`tract`) |
|--------|----------------|----------------|
| Speed | ⚡⚡⚡ Native | ⚡⚡ ~2-3x slower |
| Browser | ❌ | ✅ |
| Edge Workers | ❌ | ✅ |
| GPU | CUDA, TensorRT | ❌ |
| Bundle Size | ~50MB | ~8MB |
| Portability | Platform-specific | Universal |

**Use native** for: servers, high throughput, GPU acceleration
**Use WASM** for: browsers, edge, portability

## Building from Source

```bash
# Install wasm-pack
cargo install wasm-pack

# Build for web
wasm-pack build --target web

# Build for Node.js
wasm-pack build --target nodejs

# Build for bundlers (webpack, vite)
wasm-pack build --target bundler
```

## Use Cases

### Semantic Search

```javascript
const embedder = await createEmbedder();

// Index documents
const docs = ["Rust is fast", "Python is easy", "JavaScript runs everywhere"];
const embeddings = docs.map(d => embedder.embedOne(d));

// Search
const query = embedder.embedOne("Which language is performant?");
const scores = embeddings.map((e, i) => ({
  doc: docs[i],
  score: cosineSimilarity(query, e)
}));
scores.sort((a, b) => b.score - a.score);
console.log(scores[0]); // { doc: "Rust is fast", score: 0.82 }
```

### Text Clustering

```javascript
const texts = [
  "Machine learning is amazing",
  "Deep learning uses neural networks",
  "I love pizza",
  "Italian food is delicious"
];

const embeddings = texts.map(t => embedder.embedOne(t));
// Use k-means or hierarchical clustering on embeddings
```

### RAG (Retrieval-Augmented Generation)

```javascript
// Build knowledge base
const knowledge = [
  "RuVector is a vector database",
  "Embeddings capture semantic meaning",
  // ... more docs
];
const knowledgeEmbeddings = knowledge.map(k => embedder.embedOne(k));

// Retrieve relevant context for LLM
function getContext(query, topK = 3) {
  const queryEmb = embedder.embedOne(query);
  const scores = knowledgeEmbeddings.map((e, i) => ({
    text: knowledge[i],
    score: cosineSimilarity(queryEmb, e)
  }));
  return scores.sort((a, b) => b.score - a.score).slice(0, topK);
}
```

## Related Packages

| Package | Runtime | Use Case |
|---------|---------|----------|
| [ruvector-onnx-embeddings](https://crates.io/crates/ruvector-onnx-embeddings) | Native | High-performance servers |
| **ruvector-onnx-embeddings-wasm** | WASM | Browsers, edge, portable |

## License

MIT License - See [LICENSE](../../LICENSE) for details.

---

<p align="center">
  <b>Part of the RuVector ecosystem</b><br>
  High-performance vector operations in Rust
</p>
