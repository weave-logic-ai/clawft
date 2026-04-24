# ONNX Runtime Integration Plan for Agentic-Flow

## Executive Summary

Integrate ONNX Runtime to enable high-performance local model inference on both CPU and GPU, providing 2-100x speedup over standard inference and enabling privacy-focused, cost-free local execution of AI models including Microsoft Phi-3.

## 🎯 Objectives

1. **Performance**: Achieve 2-100x inference speedup using ONNX Runtime optimizations
2. **Hardware Flexibility**: Support both CPU and GPU execution with automatic provider selection
3. **Cost Reduction**: Enable 100% cost-free inference for local model execution
4. **Privacy**: Provide fully local inference option for sensitive data
5. **Model Support**: Support Microsoft Phi-3 and other ONNX-compatible models

## 📊 Expected Performance Gains

### CPU Optimization
- **WebAssembly + SIMD**: 3.4x performance improvement
- **ONNX Runtime CPU**: 2x average performance gain vs PyTorch/TensorFlow
- **Graph Optimizations**: 47% → 0.5% CPU usage (94% reduction)
- **Inference Speed**: ~20 tokens/second (Phi-3-medium on Intel i9-10920X)

### GPU Acceleration
- **CUDA Execution Provider**: 10-100x speedup on NVIDIA GPUs
- **TensorRT**: Additional 2-5x optimization on top of CUDA
- **DirectML (Windows)**: Native GPU acceleration on Windows
- **WebGPU**: Browser/Electron GPU acceleration

## 🏗️ Architecture

### Component Overview

```
┌─────────────────────────────────────────────────────┐
│         Agentic-Flow Multi-Model Router             │
│                                                       │
│  ┌────────────┐  ┌──────────────┐  ┌─────────────┐ │
│  │ Anthropic  │  │  OpenRouter  │  │ ONNX Runtime│ │
│  │  Provider  │  │   Provider   │  │   Provider  │ │
│  └────────────┘  └──────────────┘  └─────────────┘ │
│                                             │        │
│                                             ▼        │
│                            ┌────────────────────────┐│
│                            │  Execution Provider    ││
│                            │     Selector           ││
│                            └────────────────────────┘│
│                                       │              │
│         ┌─────────────────────────────┼──────┐      │
│         │             │               │      │      │
│         ▼             ▼               ▼      ▼      │
│    ┌────────┐   ┌─────────┐    ┌───────┐ ┌──────┐ │
│    │  CPU   │   │  CUDA   │    │ WebGPU│ │DirectML
│    │ (WASM) │   │(NVIDIA) │    │       │ │(Windows)
│    └────────┘   └─────────┘    └───────┘ └──────┘ │
└─────────────────────────────────────────────────────┘
                        │
                        ▼
            ┌───────────────────────┐
            │   ONNX Model Store    │
            │                       │
            │ • Phi-3-mini (4K)     │
            │ • Phi-3-medium (128K) │
            │ • Llama-3 ONNX       │
            │ • Custom models       │
            └───────────────────────┘
```

### Data Flow

```
User Request
    │
    ▼
Router (model selection)
    │
    ▼
ONNX Provider
    │
    ├─→ Load ONNX Model (if not cached)
    │
    ├─→ Select Execution Provider
    │   │
    │   ├─→ Probe GPU availability
    │   ├─→ Check CPU capabilities (SIMD, threads)
    │   └─→ Prioritize: CUDA > WebGPU > DirectML > CPU
    │
    ├─→ Create Inference Session
    │   │
    │   ├─→ Apply graph optimizations
    │   ├─→ Configure threading
    │   └─→ Enable SIMD if available
    │
    ├─→ Run Inference
    │   │
    │   ├─→ Tokenize input
    │   ├─→ Execute model
    │   └─→ Decode output
    │
    └─→ Return Response
        │
        ├─→ Update metrics (latency, tokens)
        └─→ Cache model for reuse
```

## 📦 NPM Packages Required

### Core Dependencies

```json
{
  "dependencies": {
    "onnxruntime-node": "^1.22.0",
    "@xenova/transformers": "^2.6.0",
    "sharp": "^0.32.0"
  },
  "devDependencies": {
    "@types/node": "^20.0.0"
  },
  "optionalDependencies": {
    "onnxruntime-node-gpu": "^1.22.0"
  }
}
```

### Package Descriptions

1. **onnxruntime-node** (Required)
   - Core ONNX Runtime for Node.js
   - CPU execution provider
   - Supports Node.js v16.x+ (recommend v20.x+)
   - Size: ~20MB

2. **onnxruntime-node-gpu** (Optional)
   - GPU acceleration via CUDA
   - Requires NVIDIA GPU + CUDA 11.8 or 12.x
   - Size: ~500MB (includes CUDA libraries)

3. **@xenova/transformers** (Helper)
   - Transformers.js for tokenization
   - Pre/post-processing utilities
   - Model download management

4. **sharp** (Optional)
   - Image processing for vision models
   - Only needed for multimodal support

## 🔧 Implementation Phases

### Phase 1: Core ONNX Provider (Week 1)

**Objective**: Basic ONNX Runtime integration with CPU inference

**Tasks**:
1. Create ONNX provider class (`src/router/providers/onnx.ts`)
2. Implement model loading and caching
3. Add CPU execution provider support
4. Integrate tokenization with Transformers.js
5. Add basic error handling

**Deliverables**:
- `ONNXProvider` class implementing `LLMProvider` interface
- Model download and caching system
- CPU inference working with Phi-3-mini

**Code Structure**:
```typescript
// src/router/providers/onnx.ts
import * as ort from 'onnxruntime-node';
import { AutoTokenizer } from '@xenova/transformers';

export class ONNXProvider implements LLMProvider {
  name = 'onnx';
  type = 'onnx' as const;
  supportsStreaming = false; // Phase 2
  supportsTools = false;     // Phase 3
  supportsMCP = false;

  private session: ort.InferenceSession | null = null;
  private tokenizer: any = null;
  private modelPath: string;

  constructor(config: ProviderConfig) {
    this.modelPath = config.models?.default || 'microsoft/Phi-3-mini-4k-instruct-onnx-cpu';
  }

  async chat(params: ChatParams): Promise<ChatResponse> {
    // Initialize session if needed
    if (!this.session) {
      await this.initializeSession();
    }

    // Tokenize input
    const inputs = await this.tokenize(params.messages);

    // Run inference
    const outputs = await this.session.run(inputs);

    // Decode output
    const response = await this.decode(outputs);

    return this.formatResponse(response, params.model);
  }

  private async initializeSession(): Promise<void> {
    // Download model if needed
    const modelPath = await this.downloadModel();

    // Create inference session with optimizations
    this.session = await ort.InferenceSession.create(modelPath, {
      executionProviders: ['cpu'],
      graphOptimizationLevel: 'all',
      enableCpuMemArena: true,
      enableMemPattern: true,
      executionMode: 'parallel',
      intraOpNumThreads: 4,
      interOpNumThreads: 2
    });

    // Initialize tokenizer
    this.tokenizer = await AutoTokenizer.from_pretrained(this.modelPath);
  }
}
```

**Testing**:
- Load Phi-3-mini ONNX model
- Run simple inference test
- Measure baseline CPU performance
- Verify memory usage < 2GB

### Phase 2: GPU Acceleration (Week 2)

**Objective**: Add GPU support with automatic provider selection

**Tasks**:
1. Implement execution provider detection
2. Add CUDA execution provider
3. Add DirectML execution provider (Windows)
4. Add WebGPU support (Electron/browser)
5. Implement automatic provider fallback

**Deliverables**:
- GPU detection and capability probing
- Multi-provider support with prioritization
- Automatic fallback chain

**Code Structure**:
```typescript
// src/router/providers/onnx.ts (additions)
export class ONNXProvider implements LLMProvider {
  private async detectExecutionProviders(): Promise<string[]> {
    const providers: string[] = [];

    // Try CUDA first (NVIDIA GPU)
    if (await this.isCUDAAvailable()) {
      providers.push('cuda');
      console.log('✅ CUDA execution provider available');
    }

    // Try DirectML (Windows GPU)
    if (process.platform === 'win32' && await this.isDirectMLAvailable()) {
      providers.push('dml');
      console.log('✅ DirectML execution provider available');
    }

    // Try WebGPU (browser/Electron)
    if (await this.isWebGPUAvailable()) {
      providers.push('webgpu');
      console.log('✅ WebGPU execution provider available');
    }

    // Always fallback to CPU
    providers.push('cpu');
    console.log('✅ CPU execution provider available');

    return providers;
  }

  private async initializeSession(): Promise<void> {
    const modelPath = await this.downloadModel();
    const providers = await this.detectExecutionProviders();

    this.session = await ort.InferenceSession.create(modelPath, {
      executionProviders: providers,
      graphOptimizationLevel: 'all',
      enableCpuMemArena: true,
      enableMemPattern: true
    });

    // Log which provider was selected
    const selectedProvider = this.session.executionProvider;
    console.log(`🚀 Using execution provider: ${selectedProvider}`);
  }

  private async isCUDAAvailable(): Promise<boolean> {
    try {
      // Check if CUDA libraries are available
      const testSession = await ort.InferenceSession.create(
        'path/to/test.onnx',
        { executionProviders: ['cuda'] }
      );
      return true;
    } catch {
      return false;
    }
  }
}
```

**Testing**:
- Test on CPU-only machine
- Test on NVIDIA GPU machine
- Test on Windows with DirectML
- Verify automatic fallback works
- Benchmark performance improvements

**Expected Results**:
- CUDA: 10-100x faster than CPU
- DirectML: 5-20x faster than CPU
- Automatic selection working

### Phase 3: Optimization & Streaming (Week 3)

**Objective**: Performance optimizations and streaming support

**Tasks**:
1. Implement streaming inference
2. Add WebAssembly SIMD optimization
3. Implement model quantization support
4. Add KV cache for faster generation
5. Optimize memory usage

**Deliverables**:
- Streaming token generation
- SIMD-optimized CPU inference
- INT8/INT4 quantized model support
- Reduced memory footprint

**Code Structure**:
```typescript
// src/router/providers/onnx.ts (streaming)
export class ONNXProvider implements LLMProvider {
  supportsStreaming = true;

  async *stream(params: ChatParams): AsyncGenerator<StreamChunk> {
    if (!this.session) {
      await this.initializeSession();
    }

    const inputs = await this.tokenize(params.messages);
    const maxTokens = params.maxTokens || 512;

    let generatedTokens = [];

    for (let i = 0; i < maxTokens; i++) {
      // Run inference for next token
      const outputs = await this.session.run({
        ...inputs,
        past_key_values: this.kvCache // Use KV cache for speed
      });

      // Extract next token
      const nextToken = this.sampleToken(outputs.logits);
      generatedTokens.push(nextToken);

      // Update KV cache
      this.updateKVCache(outputs.present_key_values);

      // Decode and yield
      const text = await this.tokenizer.decode([nextToken]);

      yield {
        type: 'content_block_delta',
        delta: {
          type: 'text_delta',
          text
        }
      };

      // Stop on EOS token
      if (nextToken === this.tokenizer.eos_token_id) {
        yield { type: 'message_stop' };
        break;
      }
    }
  }
}
```

**Optimizations**:
```typescript
// WASM + SIMD configuration
const sessionOptions: ort.InferenceSession.SessionOptions = {
  executionProviders: [{
    name: 'wasm',
    options: {
      simd: true,
      threads: navigator.hardwareConcurrency || 4
    }
  }],
  graphOptimizationLevel: 'all',
  enableCpuMemArena: true,
  enableMemPattern: true,
  executionMode: 'parallel'
};
```

**Testing**:
- Measure streaming latency
- Verify SIMD activation
- Test quantized models (INT8, INT4)
- Benchmark KV cache improvements
- Memory profiling

**Expected Results**:
- Streaming: <100ms time to first token
- SIMD: 3.4x CPU performance improvement
- Quantization: 2-4x faster inference, 50% less memory
- KV cache: 2-3x faster multi-turn conversations

### Phase 4: Model Management (Week 4)

**Objective**: Model download, caching, and selection

**Tasks**:
1. Implement HuggingFace model downloader
2. Add local model caching
3. Create model registry
4. Add model version management
5. Implement automatic model selection based on hardware

**Deliverables**:
- Automatic model download from HuggingFace
- Local model cache (~/.agentic-flow/onnx-models/)
- Model registry with hardware requirements
- Smart model selection

**Code Structure**:
```typescript
// src/router/onnx/model-manager.ts
export class ONNXModelManager {
  private cacheDir = join(homedir(), '.agentic-flow', 'onnx-models');

  private models = {
    'phi-3-mini-4k-cpu': {
      huggingface: 'microsoft/Phi-3-mini-4k-instruct-onnx-cpu',
      files: ['cpu_and_mobile/cpu-int4-rtn-block-32-acc-level-4'],
      size: '2.4GB',
      requirements: { ram: '4GB', gpu: false }
    },
    'phi-3-mini-4k-gpu': {
      huggingface: 'microsoft/Phi-3-mini-4k-instruct-onnx-cuda',
      files: ['cuda/cuda-fp16'],
      size: '4.8GB',
      requirements: { ram: '8GB', gpu: 'CUDA' }
    },
    'phi-3-medium-128k-cpu': {
      huggingface: 'microsoft/Phi-3-medium-128k-instruct-onnx-cpu',
      files: ['cpu_and_mobile/cpu-int4-rtn-block-32'],
      size: '8.2GB',
      requirements: { ram: '16GB', gpu: false }
    }
  };

  async downloadModel(modelId: string): Promise<string> {
    const modelInfo = this.models[modelId];
    if (!modelInfo) {
      throw new Error(`Unknown model: ${modelId}`);
    }

    const modelPath = join(this.cacheDir, modelId);

    // Check if already downloaded
    if (existsSync(join(modelPath, 'model.onnx'))) {
      console.log(`✅ Model ${modelId} already cached`);
      return modelPath;
    }

    console.log(`📥 Downloading ${modelId} (${modelInfo.size})...`);

    // Download from HuggingFace
    await this.downloadFromHuggingFace(
      modelInfo.huggingface,
      modelInfo.files,
      modelPath
    );

    console.log(`✅ Model ${modelId} downloaded to ${modelPath}`);
    return modelPath;
  }

  selectModelForHardware(): string {
    // Detect hardware capabilities
    const hasGPU = this.detectGPU();
    const ram = this.getAvailableRAM();

    if (hasGPU && ram >= 8) {
      return 'phi-3-mini-4k-gpu';
    } else if (ram >= 16) {
      return 'phi-3-medium-128k-cpu';
    } else {
      return 'phi-3-mini-4k-cpu';
    }
  }
}
```

**Testing**:
- Test model download from HuggingFace
- Verify caching works correctly
- Test automatic model selection
- Test with multiple models
- Verify disk space management

### Phase 5: Integration & CLI (Week 5)

**Objective**: Integrate ONNX provider into router and add CLI commands

**Tasks**:
1. Add ONNX provider to router initialization
2. Add CLI commands for ONNX management
3. Implement cost tracking (always $0 for local)
4. Add performance benchmarking
5. Update routing rules for ONNX

**Deliverables**:
- ONNX provider in multi-model router
- CLI commands for model management
- Benchmark utilities
- Updated documentation

**CLI Commands**:
```bash
# List available ONNX models
npx agentic-flow onnx models

# Download a model
npx agentic-flow onnx download phi-3-mini-4k-cpu

# List downloaded models
npx agentic-flow onnx list

# Test ONNX inference
npx agentic-flow onnx test --model phi-3-mini-4k-cpu

# Benchmark performance
npx agentic-flow onnx benchmark --model phi-3-mini-4k-cpu

# Check hardware capabilities
npx agentic-flow onnx info

# Use ONNX provider for inference
npx agentic-flow --provider onnx --model phi-3-mini-4k-cpu --task "Hello world"

# Use ONNX with GPU
npx agentic-flow --provider onnx --model phi-3-mini-4k-gpu --execution-provider cuda --task "Complex task"
```

**Router Integration**:
```typescript
// src/router/router.ts
private initializeProviders(): void {
  // ... existing providers ...

  // Initialize ONNX
  if (this.config.providers.onnx) {
    try {
      const provider = new ONNXProvider(this.config.providers.onnx);
      this.providers.set('onnx', provider);
      console.log('✅ ONNX provider initialized');
    } catch (error) {
      console.error('❌ Failed to initialize ONNX:', error);
    }
  }
}
```

**Configuration**:
```json
{
  "providers": {
    "onnx": {
      "models": {
        "default": "phi-3-mini-4k-cpu",
        "fast": "phi-3-mini-4k-cpu",
        "advanced": "phi-3-medium-128k-cpu",
        "gpu": "phi-3-mini-4k-gpu"
      },
      "executionProviders": ["cuda", "dml", "cpu"],
      "graphOptimizationLevel": "all",
      "enableMemoryOptimization": true,
      "threads": 4,
      "timeout": 60000
    }
  },
  "routing": {
    "rules": [
      {
        "condition": {
          "privacy": "high",
          "localOnly": true
        },
        "action": {
          "provider": "onnx",
          "model": "phi-3-mini-4k-cpu"
        },
        "reason": "Privacy-sensitive tasks use local ONNX models"
      },
      {
        "condition": {
          "agentType": ["researcher"],
          "complexity": "low"
        },
        "action": {
          "provider": "onnx",
          "model": "phi-3-mini-4k-cpu"
        },
        "reason": "Simple tasks use free local inference"
      }
    ]
  }
}
```

### Phase 6: Advanced Features (Week 6)

**Objective**: Vision support, tool calling, and production optimizations

**Tasks**:
1. Add multimodal support (vision)
2. Implement tool calling for ONNX models
3. Add model fine-tuning support
4. Implement distributed inference
5. Production hardening

**Features**:
- Phi-3-vision for image understanding
- Custom tool calling layer
- Model adaptation for specific tasks
- Multi-GPU support
- Load balancing across models

## 📋 Configuration Examples

### Basic CPU Configuration

```json
{
  "version": "1.0",
  "defaultProvider": "onnx",
  "providers": {
    "onnx": {
      "models": {
        "default": "phi-3-mini-4k-cpu"
      },
      "executionProviders": ["cpu"],
      "threads": 4
    }
  }
}
```

### GPU Optimized Configuration

```json
{
  "version": "1.0",
  "defaultProvider": "onnx",
  "providers": {
    "onnx": {
      "models": {
        "default": "phi-3-mini-4k-gpu"
      },
      "executionProviders": ["cuda", "cpu"],
      "cudaOptions": {
        "deviceId": 0,
        "gpuMemLimit": 4294967296,
        "arenExtendStrategy": "kSameAsRequested"
      }
    }
  }
}
```

### Hybrid Cloud + Local Configuration

```json
{
  "version": "1.0",
  "defaultProvider": "anthropic",
  "fallbackChain": ["anthropic", "openrouter", "onnx"],
  "providers": {
    "anthropic": { ... },
    "openrouter": { ... },
    "onnx": {
      "models": {
        "default": "phi-3-mini-4k-cpu"
      }
    }
  },
  "routing": {
    "mode": "rule-based",
    "rules": [
      {
        "condition": { "privacy": "high" },
        "action": { "provider": "onnx" }
      },
      {
        "condition": { "complexity": "low" },
        "action": { "provider": "onnx" }
      },
      {
        "condition": { "complexity": "high" },
        "action": { "provider": "anthropic" }
      }
    ]
  }
}
```

## 🎯 Success Metrics

### Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| CPU Inference Speed | 15-20 tokens/sec | Phi-3-mini on i9-10920X |
| GPU Inference Speed | 100+ tokens/sec | Phi-3-mini on RTX 3090 |
| Time to First Token | <500ms | Streaming mode |
| Memory Usage (CPU) | <4GB | Phi-3-mini INT4 |
| Model Load Time | <10s | First request only |

### Cost Savings

| Scenario | Cloud Cost | ONNX Cost | Savings |
|----------|-----------|-----------|---------|
| 1M tokens (research) | $3.00 | $0.00 | 100% |
| 1M tokens (coding) | $15.00 | $0.00 | 100% |
| Monthly development | $100-500 | $0.00 | 100% |

### Quality Targets

| Metric | Target |
|--------|--------|
| Accuracy vs Cloud | >95% for simple tasks |
| Success Rate | >99% (no network failures) |
| Latency Variance | <10% (consistent) |

## 🔒 Security & Privacy

### Benefits
- ✅ No data sent to external services
- ✅ No API keys required for local models
- ✅ Fully offline operation possible
- ✅ HIPAA/GDPR compliant by design
- ✅ No usage tracking or telemetry

### Considerations
- Models downloaded from HuggingFace (verify checksums)
- Model license compliance (MIT for Phi-3)
- Disk space for model storage (2-10GB per model)

## 🐛 Known Limitations

1. **Model Size**: ONNX models are 2-10GB, requiring significant disk space
2. **Initial Download**: First-time model download takes 5-30 minutes
3. **Hardware Requirements**: GPU models require NVIDIA GPU with CUDA
4. **Tool Calling**: Limited compared to Claude/GPT (requires custom implementation)
5. **Streaming**: Initial implementation may have higher latency than cloud
6. **Context Length**: Phi-3-mini limited to 4K tokens (vs 200K for Claude)

## 📊 Benchmarking Plan

### Test Suite

```bash
# CPU Benchmark
npx agentic-flow onnx benchmark \
  --model phi-3-mini-4k-cpu \
  --provider cpu \
  --iterations 100

# GPU Benchmark
npx agentic-flow onnx benchmark \
  --model phi-3-mini-4k-gpu \
  --provider cuda \
  --iterations 100

# Comparison Benchmark
npx agentic-flow router benchmark \
  --providers "onnx,anthropic,openrouter" \
  --task "Write a hello world function" \
  --iterations 50
```

### Benchmark Metrics

1. **Latency**
   - Time to first token
   - Tokens per second
   - End-to-end request time

2. **Throughput**
   - Concurrent requests
   - Batch processing

3. **Resource Usage**
   - CPU utilization
   - Memory consumption
   - GPU memory
   - Disk I/O

4. **Quality**
   - Response accuracy
   - Instruction following
   - Consistency

## 🚀 Deployment Strategy

### Development Phase
- Use ONNX CPU for testing
- Small models (Phi-3-mini)
- Local development only

### Staging Phase
- Test GPU acceleration
- Larger models (Phi-3-medium)
- Performance benchmarking

### Production Phase
- Hybrid cloud + local routing
- GPU for high-throughput
- Fallback to cloud for complex tasks

## 📚 Resources

### Documentation
- ONNX Runtime: https://onnxruntime.ai/
- Phi-3 Models: https://huggingface.co/microsoft/Phi-3-mini-4k-instruct-onnx-cpu
- Transformers.js: https://huggingface.co/docs/transformers.js

### Examples
- Phi-3 Chat: https://github.com/microsoft/onnxruntime-inference-examples/tree/main/js/chat
- WebGPU RAG: https://github.com/microsoft/Phi-3CookBook/tree/main/code/08.RAG/rag_webgpu_chat

### Performance Guides
- CPU Optimization: https://onnxruntime.ai/docs/performance/model-optimizations/graph-optimizations.html
- GPU Providers: https://onnxruntime.ai/docs/execution-providers/

## ✅ Next Steps

1. **Immediate**: Review and approve this implementation plan
2. **Week 1**: Begin Phase 1 (Core ONNX Provider)
3. **Week 2**: Implement Phase 2 (GPU Acceleration)
4. **Week 3**: Complete Phase 3 (Optimization)
5. **Week 4**: Execute Phase 4 (Model Management)
6. **Week 5**: Finish Phase 5 (Integration)
7. **Week 6**: Deploy Phase 6 (Advanced Features)

---

**Status**: Ready for Implementation
**Estimated Timeline**: 6 weeks
**Estimated Effort**: 120-150 hours
**Risk Level**: Low (proven technology, clear path)
**ROI**: High (100% cost savings for local inference, 2-100x performance improvement)
