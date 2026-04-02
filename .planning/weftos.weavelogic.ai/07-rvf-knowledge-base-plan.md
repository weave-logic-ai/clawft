# RVF Knowledge Base for Playground Tour Guide

## Overview

Ship a purpose-built vector knowledge base as an RVF segment file (`weftos-docs.rvf.json`) alongside the WASM playground. The agent uses it for RAG — retrieving relevant documentation chunks to answer questions accurately, point to the right docs, and serve as the guided tour's knowledge backbone.

This is also a product demo: the KB uses WeftOS's own RVF format with a WITNESS chain proving tamper-evidence. The tour guide is itself a demonstration of the provenance system it's teaching visitors about.

## Architecture

### Build-Time Pipeline

A Rust CLI tool (`scripts/build-kb/`) that:

1. **Reads the corpus** — 68 MDX doc pages, code examples, CLI help, architecture docs
2. **Chunks intelligently** — by heading section (H2/H3 boundaries), ~200-500 tokens per chunk
3. **Generates embeddings** — using the existing ONNX all-MiniLM-L6-v2 model (384-dim)
4. **Builds RVF segment file** — using `SegmentFileBuilder` from `clawft-core::embeddings::rvf_io`
5. **Attaches WITNESS chain** — tamper-evident hash chain over all segments
6. **Outputs** — `docs/src/public/kb/weftos-docs.rvf.json`

```rust
// Pseudocode for the build script
use clawft_core::embeddings::rvf_io::*;
use clawft_core::embeddings::witness::WitnessChain;

let mut builder = SegmentFileBuilder::new("tour-guide", "docs")
    .dimension(384)
    .embedder_name("all-MiniLM-L6-v2");

let mut witness = WitnessChain::new();

for chunk in corpus.chunks() {
    let embedding = onnx_model.embed(&chunk.text);
    let segment = make_segment(
        &chunk.id,
        &chunk.text,
        embedding,
        serde_json::json!({
            "source": chunk.doc_path,      // "/docs/clawft/providers"
            "title": chunk.page_title,     // "LLM Providers"
            "section": chunk.section,      // "Provider Routing"
            "doc_url": chunk.url,          // full URL for linking
            "category": chunk.category,    // "configuration" | "architecture" | "guide"
        }),
        chunk.tags.clone(),
        "docs",
        "all-MiniLM-L6-v2",
    );
    
    witness.append(WitnessOperation::Store, chunk.id.as_bytes());
    builder = builder.add_segment(segment);
}

let file = builder.witness_chain(witness).build();
write_segment_file(&file, Path::new("docs/src/public/kb/weftos-docs.rvf.json"))?;
```

### Corpus Sources

| Source | Content | Namespace | Est. Chunks |
|--------|---------|-----------|-------------|
| `content/docs/clawft/*.mdx` | Agent runtime docs (16 pages) | `docs` | ~80 |
| `content/docs/weftos/*.mdx` | Kernel docs (52 pages) | `docs` | ~220 |
| Code examples from docs | Working configs, CLI invocations | `examples` | ~50 |
| `weft --help` + subcommand help | All CLI commands and flags | `cli` | ~20 |
| Crate-level rustdoc comments | Public API descriptions from `lib.rs` | `api` | ~50 |
| Architecture overview | Layer descriptions, crate dependencies | `architecture` | ~30 |
| FAQ content | From docs + anticipated questions | `faq` | ~30 |
| Install instructions | All paths with expected output | `getting-started` | ~20 |

**Total: ~500 segments**

### Excluded Content (NOT in KB)

- `.planning/` — internal planning docs
- `.weftos/` — session state, development memory
- Sprint notes, symposium transcripts
- ADRs (unless published in docs site)
- Git history, commit messages
- API keys, environment variables, credentials
- Test files (except as documentation examples)

### Segment Metadata Schema

Each `MemorySegment.metadata` contains:

```json
{
  "source": "/docs/clawft/providers",
  "title": "LLM Providers",
  "section": "Provider Routing Architecture",
  "doc_url": "https://weftos.weavelogic.ai/docs/clawft/providers#provider-routing",
  "category": "configuration",
  "chunk_index": 3,
  "total_chunks": 7,
  "has_code": true,
  "language": "toml"
}
```

### Tags Taxonomy

| Tag | Meaning |
|-----|---------|
| `clawft` | Agent runtime layer |
| `weftos` | Kernel layer |
| `ecc` | Cognitive layer |
| `install` | Installation/setup |
| `config` | Configuration |
| `providers` | LLM providers |
| `tools` | Agent tools |
| `governance` | Constitutional governance |
| `security` | Security features |
| `mesh` | Mesh networking |
| `provenance` | ExoChain audit trail |
| `code-example` | Contains runnable code |
| `cli` | CLI command reference |

## Browser Runtime

### Loading the KB

```typescript
// In WasmPlayground.tsx
const loadKnowledgeBase = async () => {
  const response = await fetch('/kb/weftos-docs.rvf.json');
  const segmentFile = await response.json();
  
  // Verify WITNESS chain (optional but demonstrates provenance)
  if (segmentFile.witness_chain) {
    const valid = verifyWitnessChain(segmentFile.witness_chain);
    console.log(`[tour-guide] KB integrity: ${valid ? 'VERIFIED' : 'FAILED'}`);
  }
  
  return segmentFile.segments; // Array of MemorySegment
};
```

### Search (Cosine Similarity in JS)

For ~500 segments at 384 dims, brute-force cosine search takes ~1ms. No HNSW needed in browser.

```typescript
function cosineSimilarity(a: number[], b: number[]): number {
  let dot = 0, normA = 0, normB = 0;
  for (let i = 0; i < a.length; i++) {
    dot += a[i] * b[i];
    normA += a[i] * a[i];
    normB += b[i] * b[i];
  }
  return dot / (Math.sqrt(normA) * Math.sqrt(normB));
}

function searchKB(queryEmbedding: number[], segments: MemorySegment[], topK = 5) {
  return segments
    .map(seg => ({ segment: seg, score: cosineSimilarity(queryEmbedding, seg.embedding) }))
    .sort((a, b) => b.score - a.score)
    .slice(0, topK);
}
```

### Query-Time Embedding

Two approaches:

**Option A: Transformers.js (recommended for quality)**
- Load all-MiniLM-L6-v2 in browser via `@xenova/transformers`
- ~30MB model download (cached by browser)
- Same model as build-time = perfect embedding alignment
- ~50ms per query embedding

**Option B: Keyword/BM25 fallback (MVP)**
- No model download needed
- Pre-compute TF-IDF at build time, ship alongside embeddings
- Lower quality but instant and lightweight

**Recommendation**: Start with Option B for MVP (zero download), add Option A as a toggle ("Enhanced Search: downloads 30MB model for better results").

### RAG Injection

```typescript
const systemPrompt = `You are the WeftOS Tour Guide, an expert on the WeftOS AI framework.
You answer questions using ONLY the knowledge provided in the context below.
When referencing documentation, include the link in your response.
If you don't know something, say so — don't guess.

## Relevant Documentation

${topKResults.map(r => `### ${r.segment.metadata.title} — ${r.segment.metadata.section}
Source: ${r.segment.metadata.doc_url}
${r.segment.text}
`).join('\n')}
`;
```

## Build Integration

### New Script: `scripts/build-kb.sh`

```bash
#!/usr/bin/env bash
# Build the RVF knowledge base for the playground tour guide.
# Requires: cargo, ONNX model at .weftos/models/all-MiniLM-L6-v2.onnx

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "Building knowledge base..."
cargo run --release -p build-kb -- \
    --docs-dir "$ROOT/docs/src/content/docs" \
    --model "$ROOT/.weftos/models/all-MiniLM-L6-v2.onnx" \
    --output "$ROOT/docs/src/public/kb/weftos-docs.rvf.json" \
    --cli-binary "$ROOT/target/release/weft"

echo "Knowledge base built: $(wc -c < "$ROOT/docs/src/public/kb/weftos-docs.rvf.json") bytes"
echo "Segments: $(jq '.header.segment_count' "$ROOT/docs/src/public/kb/weftos-docs.rvf.json")"
```

### New Crate: `scripts/build-kb/` (or `tools/build-kb/`)

A small Rust binary that:
1. Walks the MDX docs directory
2. Parses MDX → extracts text by heading sections
3. Strips frontmatter, MDX components, import statements
4. Generates embeddings via `ort` (ONNX Runtime)
5. Builds the RVF segment file with WITNESS chain
6. Writes to output path

This crate depends on:
- `clawft-core` (for `rvf_io`, `witness`)
- `ort` (for ONNX inference)
- `pulldown-cmark` (for Markdown parsing — already in workspace)
- `walkdir` or `glob` (for file discovery)

### CI Integration

Add to `scripts/build.sh`:
```bash
cmd_kb() {
    header "Building RVF knowledge base"
    timer_start
    scripts/build-kb.sh
    timer_end
}
```

Add to release pipeline: rebuild KB on every docs change, include in Vercel deploy.

## Size Estimates

| Component | Estimated Size |
|-----------|---------------|
| 500 segments × text (~200 tokens avg) | ~400 KB |
| 500 segments × 384 floats × 4 bytes | ~768 KB |
| Metadata + tags + headers | ~100 KB |
| WITNESS chain | ~16 KB |
| **Total (uncompressed JSON)** | **~1.3 MB** |
| **Gzipped** | **~400-600 KB** |

Vercel automatically gzip-compresses static assets, so the actual transfer is ~500KB.

## WITNESS Chain as Product Demo

The WITNESS chain on the KB file is itself a demo of WeftOS provenance:

```
Tour Guide: "Every piece of knowledge I use is cryptographically linked.
This knowledge base was built with a WITNESS chain — the same tamper-evident
system that WeftOS uses for agent audit trails. You can verify that no
documentation was altered after embedding."

[Show WITNESS verification ✓ in the UI]
```

This ties the tour guide's own infrastructure to the provenance story in Step 6 of the guided tour. Meta-demonstration.

## Iterative Improvement

### v1 (MVP): Static KB + keyword search
- Pre-built RVF file shipped as static asset
- BM25/keyword search in browser (no model download)
- Rebuild manually when docs change

### v2: Semantic search + Transformers.js
- Add `@xenova/transformers` for in-browser embeddings
- Same model alignment as build-time embeddings
- Toggle: "Enhanced Search" downloads 30MB model

### v3: Live KB updates
- KB auto-rebuilds in CI on docs changes
- Version stamped in RVF header
- Browser checks for KB updates on playground load

### v4: User-contributed context
- Users can add their own context to the KB (project files, docs)
- Agent personalizes answers based on their stack
- This becomes the "try the assessment on your own code" flow

### v5: Full ECC integration
- KB feeds into a browser-side ECC causal graph
- Tour guide shows causal relationships between concepts
- Knowledge graph visualization backed by real data

## Open Questions

1. Should the build-kb tool be a standalone binary crate or a cargo script?
2. Should we quantize embeddings (f32 → f16 or int8) to halve the file size?
3. Should the RVF file be split into multiple files by namespace for lazy loading?
4. Should we pre-compute common query embeddings (install, quickstart, providers, etc.) to avoid needing any embedding model in browser for common questions?
