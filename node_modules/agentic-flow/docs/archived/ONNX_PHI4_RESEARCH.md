# ONNX Runtime Integration Research - Phi-4 Implementation

**Date**: 2025-10-03
**Status**: Research Complete - Implementation Ready

## Executive Summary

Research findings for integrating Microsoft's Phi-4-mini-instruct-onnx model with agentic-flow using onnxruntime-node for CPU inference.

## Key Findings

### 1. Library Comparison

| Library | Use Case | Performance | Node.js Support | Status |
|---------|----------|-------------|-----------------|--------|
| **onnxruntime-node** | Server-side inference | **Fastest** | ✅ Native | **Recommended** |
| onnxruntime-web | Browser/frontend | Good | ✅ WebAssembly | Not suitable for CLI |
| @xenova/transformers | Simplified API | Moderate | ✅ Yes | Limited model support |
| onnxruntime-genai | Generative AI | Excellent | ❌ Python only | Not available for Node.js |

**Conclusion**: Use **onnxruntime-node** v1.22.0 - it's the official Microsoft library with best performance for server-side Node.js applications.

### 2. Phi-4-mini-instruct-onnx Model Details

**HuggingFace**: https://huggingface.co/microsoft/Phi-4-mini-instruct-onnx

#### Model Specifications
- **Context Length**: 128K tokens
- **License**: MIT
- **Quantization**: INT4 (RTN - Round To Nearest)
- **Variants**:
  - `cpu-int4-rtn-block-32-acc-level-4` - CPU optimized
  - `gpu-int4-rtn-block-32` - CUDA optimized

#### Performance Characteristics
- **Speedup**: 12.4x faster than PyTorch on CPU
- **Memory**: Reduced via INT4 quantization
- **Platform**: Cross-platform (Windows, Linux, macOS)

### 3. Key Challenges Identified

#### Challenge 1: onnxruntime-genai Not Available for Node.js
- **Issue**: The `onnxruntime-genai` library (used in Python examples) has no npm package
- **Impact**: Cannot use simplified GenAI API in Node.js
- **Solution**: Use onnxruntime-node directly with manual tokenization

#### Challenge 2: Transformers.js Incompatibility
- **Issue**: @xenova/transformers doesn't support Phi-4 models (only GPT-2, Llama, etc.)
- **Error**: "Unsupported model type: phi3"
- **Solution**: Bypass transformers.js, use onnxruntime-node + custom tokenizer

#### Challenge 3: Manual Tokenization Required
- **Issue**: Need to implement Phi-4 chat template and tokenization
- **Required**:
  - Tokenizer model (tokenizer.json)
  - Chat template formatting
  - Pre/post processing
- **Solution**: Use HuggingFace tokenizers library or implement manually

### 4. Recommended Architecture

```
┌─────────────────────────────────────────────┐
│         ONNXProvider (Updated)              │
├─────────────────────────────────────────────┤
│                                             │
│  ┌───────────────────────────────────┐     │
│  │   onnxruntime-node v1.22.0       │     │
│  │   (InferenceSession)              │     │
│  └───────────────────────────────────┘     │
│              ↓                              │
│  ┌───────────────────────────────────┐     │
│  │   Phi-4 ONNX Model                │     │
│  │   cpu-int4-rtn-block-32           │     │
│  └───────────────────────────────────┘     │
│              ↓                              │
│  ┌───────────────────────────────────┐     │
│  │   Custom Tokenizer                │     │
│  │   (Phi-4 chat template)           │     │
│  └───────────────────────────────────┘     │
│                                             │
└─────────────────────────────────────────────┘
```

### 5. Implementation Plan

#### Phase 1: Download Phi-4 Model ✅
```bash
huggingface-cli download microsoft/Phi-4-mini-instruct-onnx \
  --include cpu-int4-rtn-block-32-acc-level-4/* \
  --local-dir ./models/phi-4
```

#### Phase 2: Install Dependencies
```bash
npm install onnxruntime-node@^1.22.0
npm install @huggingface/tokenizers  # For tokenization
```

#### Phase 3: Implement ONNXProvider
- Use onnxruntime-node InferenceSession API
- Load Phi-4 ONNX model from disk
- Implement Phi-4 chat template
- Handle tokenization/detokenization
- Support CPU execution provider (upgradeable to CUDA)

#### Phase 4: Chat Template Format
```typescript
// Phi-4 uses the following chat format:
// <|system|>
// {system_message}<|end|>
// <|user|>
// {user_message}<|end|>
// <|assistant|>
// {assistant_response}<|end|>
```

### 6. API Comparison

#### Python (onnxruntime-genai) - NOT AVAILABLE FOR NODE
```python
import onnxruntime_genai as og
model = og.Model("cpu-int4-rtn-block-32-acc-level-4")
tokenizer = og.Tokenizer(model)
params = og.GeneratorParams(model)
generator = og.Generator(model, params)
```

#### Node.js (onnxruntime-node) - RECOMMENDED
```typescript
import * as ort from 'onnxruntime-node';

// Load model
const session = await ort.InferenceSession.create(
  './models/phi-4/model.onnx',
  { executionProviders: ['cpu'] }
);

// Manual tokenization
const inputIds = tokenize(prompt);
const feeds = { input_ids: new ort.Tensor('int64', inputIds, [1, inputIds.length]) };

// Run inference
const results = await session.run(feeds);
const outputIds = results.output.data;
const text = detokenize(outputIds);
```

### 7. Performance Targets

| Metric | Target | Expected |
|--------|--------|----------|
| **First Token Latency** | <2000ms | ~1500ms |
| **Tokens/Second** | >15 | 15-25 |
| **Memory Usage** | <4GB | ~2-3GB |
| **Cost** | $0 | FREE |

### 8. Execution Providers

| Provider | Platform | Support | Acceleration |
|----------|----------|---------|--------------|
| CPU | All | ✅ Default | AVX2, AVX512 |
| CUDA | Linux + NVIDIA | ✅ Available | 10-100x |
| DirectML | Windows | ✅ Available | 5-20x |
| CoreML | macOS | ⚠️ Experimental | 5-10x |

### 9. Model Download Strategy

**Option 1: Manual Download (Recommended)**
- Use huggingface-cli to pre-download model
- Store in `./models/phi-4/` directory
- Faster initialization, no runtime downloads

**Option 2: Automatic Download**
- Use @huggingface/hub library
- Download on first run
- Slower first initialization

**Recommendation**: Pre-download for Docker deployments, auto-download for development.

### 10. Docker Considerations

```dockerfile
# In Dockerfile
RUN pip install huggingface-hub
RUN huggingface-cli download microsoft/Phi-4-mini-instruct-onnx \
    --include cpu-int4-rtn-block-32-acc-level-4/* \
    --local-dir /app/models/phi-4

# Or mount as volume
volumes:
  - ./models:/app/models
```

## Next Steps

1. ✅ Research complete - onnxruntime-node confirmed as best option
2. 🔄 Download Phi-4 model files
3. ⏳ Implement ONNXProvider with onnxruntime-node
4. ⏳ Create tokenizer integration
5. ⏳ Test in Docker CPU environment
6. ⏳ Benchmark performance
7. ⏳ Add GPU support (CUDA/DirectML)

## Resources

- ONNX Runtime Node.js: https://onnxruntime.ai/docs/api/js/
- Phi-4 Model: https://huggingface.co/microsoft/Phi-4-mini-instruct-onnx
- ONNX Runtime GenAI: https://github.com/microsoft/onnxruntime-genai (Python reference)
- HuggingFace Tokenizers: https://www.npmjs.com/package/@huggingface/tokenizers

## Conclusion

**onnxruntime-node is the correct choice** for implementing Phi-4 inference in agentic-flow:
- Official Microsoft library
- Best performance for Node.js
- CPU and GPU support
- Production-ready

**Note**: We'll need to implement manual tokenization since onnxruntime-genai (with built-in tokenization) is Python-only.
