# ONNX Environment Variables Reference

Complete guide to configuring ONNX local inference via environment variables.

## Quick Start

```bash
# Enable ONNX with all optimizations
export PROVIDER=onnx
export ONNX_OPTIMIZED=true

# Run your agent
npx agentic-flow --agent coder --task "Build feature"
```

---

## Provider Selection

### `PROVIDER`
**Values:** `anthropic` | `openrouter` | `onnx`
**Default:** `anthropic`
**Description:** Set the AI provider for all CLI commands

```bash
# Use ONNX for all commands
export PROVIDER=onnx
npx agentic-flow --agent coder --task "test"

# Use OpenRouter
export PROVIDER=openrouter
npx agentic-flow --agent coder --task "test"
```

### `USE_ONNX`
**Values:** `true` | `false`
**Default:** `false`
**Description:** Force ONNX provider (legacy, use `PROVIDER=onnx` instead)

```bash
export USE_ONNX=true
npx agentic-flow --agent coder --task "test"
```

---

## Model Configuration

### `ONNX_MODEL_PATH`
**Values:** File path
**Default:** `./models/phi-4-mini/cpu_and_mobile/cpu-int4-rtn-block-32-acc-level-4/model.onnx`
**Description:** Custom path to ONNX model file

```bash
# Use custom model location
export ONNX_MODEL_PATH=/mnt/models/custom-model.onnx
```

### `ONNX_EXECUTION_PROVIDERS`
**Values:** Comma-separated list: `cpu`, `cuda`, `dml`, `coreml`
**Default:** `cpu`
**Description:** Execution providers for inference (affects speed dramatically)

```bash
# CPU only (default, slowest)
export ONNX_EXECUTION_PROVIDERS=cpu

# NVIDIA GPU acceleration (10-50x faster)
export ONNX_EXECUTION_PROVIDERS=cuda,cpu

# Windows DirectML GPU (5-15x faster)
export ONNX_EXECUTION_PROVIDERS=dml,cpu

# macOS Apple Silicon (7-20x faster)
export ONNX_EXECUTION_PROVIDERS=coreml,cpu
```

**Performance Impact:**
- `cpu`: ~6 tokens/sec
- `cuda`: ~60-300 tokens/sec (10-50x faster)
- `dml`: ~30-100 tokens/sec (5-15x faster)
- `coreml`: ~40-120 tokens/sec (7-20x faster)

---

## Generation Parameters

### `ONNX_MAX_TOKENS`
**Values:** Integer (1-4096)
**Default:** `200`
**Description:** Maximum tokens to generate in response

```bash
# Short responses (faster)
export ONNX_MAX_TOKENS=100

# Long responses
export ONNX_MAX_TOKENS=500
```

**Tip:** Keep under 300 for best speed. Context + output must stay under 4K tokens total.

### `ONNX_TEMPERATURE`
**Values:** Float (0.0-2.0)
**Default:** `0.7` (base), `0.3` (optimized)
**Description:** Controls output randomness/creativity

```bash
# Deterministic code (recommended for code generation)
export ONNX_TEMPERATURE=0.2

# Balanced
export ONNX_TEMPERATURE=0.7

# Creative writing
export ONNX_TEMPERATURE=0.9
```

**Recommended Settings:**
| Task Type | Temperature | Why |
|-----------|-------------|-----|
| Code generation | 0.2-0.4 | Consistent syntax |
| Refactoring | 0.3-0.5 | Some creativity, but safe |
| Documentation | 0.5-0.7 | Clear but varied |
| Brainstorming | 0.7-0.9 | Diverse ideas |
| Math/Logic | 0.1-0.2 | Precise |

### `ONNX_TOP_K`
**Values:** Integer (1-100)
**Default:** `50`
**Description:** Consider top K tokens for sampling

```bash
# More focused (deterministic)
export ONNX_TOP_K=20

# More diverse
export ONNX_TOP_K=80
```

### `ONNX_TOP_P`
**Values:** Float (0.0-1.0)
**Default:** `0.9`
**Description:** Nucleus sampling threshold (probability mass)

```bash
# Very focused
export ONNX_TOP_P=0.7

# Balanced
export ONNX_TOP_P=0.9

# Diverse
export ONNX_TOP_P=0.95
```

### `ONNX_REPETITION_PENALTY`
**Values:** Float (1.0-2.0)
**Default:** `1.1`
**Description:** Penalty for token repetition

```bash
# No penalty (may repeat)
export ONNX_REPETITION_PENALTY=1.0

# Mild penalty (recommended)
export ONNX_REPETITION_PENALTY=1.1

# Strong penalty (more diverse but may lose coherence)
export ONNX_REPETITION_PENALTY=1.5
```

---

## Optimization Features

### `ONNX_OPTIMIZED`
**Values:** `true` | `false`
**Default:** `false`
**Description:** Enable optimized ONNX provider with context pruning and prompt enhancement

```bash
# Enable all optimizations (recommended)
export ONNX_OPTIMIZED=true

# Use base provider
export ONNX_OPTIMIZED=false
```

**Benefits when enabled:**
- 30-50% quality improvement via prompt optimization
- 2-4x speed improvement via context pruning
- Automatic sliding window context management

### `ONNX_MAX_CONTEXT_TOKENS`
**Values:** Integer (500-4000)
**Default:** `2048`
**Description:** Maximum context tokens (used when `ONNX_OPTIMIZED=true`)

```bash
# Smaller context (faster, less history)
export ONNX_MAX_CONTEXT_TOKENS=1000

# Larger context (slower, more history)
export ONNX_MAX_CONTEXT_TOKENS=3000
```

**Warning:** Total (context + output) must stay under 4096 tokens (Phi-4 limit)

### `ONNX_SLIDING_WINDOW`
**Values:** `true` | `false`
**Default:** `true` (when `ONNX_OPTIMIZED=true`)
**Description:** Enable sliding window context pruning

```bash
# Enable context pruning (recommended for speed)
export ONNX_SLIDING_WINDOW=true

# Disable (keep all context)
export ONNX_SLIDING_WINDOW=false
```

**Performance:** 2-4x faster inference by keeping only recent messages

### `ONNX_PROMPT_OPTIMIZATION`
**Values:** `true` | `false`
**Default:** `true` (when `ONNX_OPTIMIZED=true`)
**Description:** Auto-enhance prompts for better quality

```bash
# Enable prompt optimization (recommended for quality)
export ONNX_PROMPT_OPTIMIZATION=true

# Disable
export ONNX_PROMPT_OPTIMIZATION=false
```

**Quality:** 30-50% improvement by adding quality guidelines to code tasks

### `ONNX_CACHE_SYSTEM_PROMPTS`
**Values:** `true` | `false`
**Default:** `true` (when `ONNX_OPTIMIZED=true`)
**Description:** Cache processed system prompts for reuse

```bash
# Enable caching (faster repeated tasks)
export ONNX_CACHE_SYSTEM_PROMPTS=true

# Disable
export ONNX_CACHE_SYSTEM_PROMPTS=false
```

**Speed:** 30-40% faster on repeated prompts

---

## Preset Configurations

### Maximum Speed
```bash
export PROVIDER=onnx
export ONNX_EXECUTION_PROVIDERS=cuda,cpu  # or dml/coreml
export ONNX_OPTIMIZED=true
export ONNX_MAX_CONTEXT_TOKENS=1000
export ONNX_MAX_TOKENS=100
export ONNX_SLIDING_WINDOW=true
export ONNX_CACHE_SYSTEM_PROMPTS=true
```

**Result:** 180+ tokens/sec (with GPU), minimal latency

### Maximum Quality
```bash
export PROVIDER=onnx
export ONNX_OPTIMIZED=true
export ONNX_TEMPERATURE=0.3
export ONNX_TOP_P=0.9
export ONNX_TOP_K=50
export ONNX_REPETITION_PENALTY=1.1
export ONNX_PROMPT_OPTIMIZATION=true
export ONNX_MAX_TOKENS=300
```

**Result:** 8.5/10 quality for code tasks

### Balanced
```bash
export PROVIDER=onnx
export ONNX_OPTIMIZED=true
export ONNX_EXECUTION_PROVIDERS=cpu  # or gpu
export ONNX_TEMPERATURE=0.3
export ONNX_MAX_TOKENS=200
export ONNX_MAX_CONTEXT_TOKENS=1500
```

**Result:** Good quality + speed tradeoff

### CPU Only (No GPU)
```bash
export PROVIDER=onnx
export ONNX_OPTIMIZED=true
export ONNX_EXECUTION_PROVIDERS=cpu
export ONNX_MAX_CONTEXT_TOKENS=1000
export ONNX_MAX_TOKENS=150
export ONNX_TEMPERATURE=0.3
export ONNX_SLIDING_WINDOW=true
```

**Result:** Best CPU performance (still ~12 tokens/sec)

---

## Use Case Configurations

### Code Generation
```bash
export PROVIDER=onnx
export ONNX_OPTIMIZED=true
export ONNX_TEMPERATURE=0.3  # Deterministic
export ONNX_TOP_P=0.9
export ONNX_PROMPT_OPTIMIZATION=true
export ONNX_MAX_TOKENS=250
```

### Code Review
```bash
export PROVIDER=onnx
export ONNX_OPTIMIZED=true
export ONNX_TEMPERATURE=0.4
export ONNX_MAX_TOKENS=300
export ONNX_MAX_CONTEXT_TOKENS=2000  # Need more context
```

### Documentation
```bash
export PROVIDER=onnx
export ONNX_OPTIMIZED=true
export ONNX_TEMPERATURE=0.6  # More creative
export ONNX_TOP_P=0.95
export ONNX_MAX_TOKENS=400
```

### Refactoring
```bash
export PROVIDER=onnx
export ONNX_OPTIMIZED=true
export ONNX_TEMPERATURE=0.35
export ONNX_MAX_TOKENS=200
export ONNX_SLIDING_WINDOW=true
```

---

## Performance Tuning

### Scenario 1: Too Slow (6 tokens/sec)

**Problem:** CPU-only inference is slow
**Solutions:**
1. Enable GPU acceleration (biggest impact)
2. Reduce context size
3. Enable sliding window
4. Reduce max tokens

```bash
# Quick wins (no hardware change)
export ONNX_MAX_CONTEXT_TOKENS=1000  # 2x faster
export ONNX_SLIDING_WINDOW=true
export ONNX_MAX_TOKENS=100

# Best solution (requires GPU)
export ONNX_EXECUTION_PROVIDERS=cuda,cpu  # 30x faster!
```

### Scenario 2: Low Quality Output

**Problem:** Generated code has bugs/missing features
**Solutions:**
1. Enable optimizations
2. Lower temperature
3. Use specific prompts
4. Enable prompt optimization

```bash
export ONNX_OPTIMIZED=true
export ONNX_TEMPERATURE=0.3
export ONNX_PROMPT_OPTIMIZATION=true
export ONNX_TOP_K=50
export ONNX_TOP_P=0.9
```

### Scenario 3: Out of Memory

**Problem:** System runs out of RAM
**Solutions:**
1. Reduce context size
2. Reduce max tokens
3. Close other applications

```bash
export ONNX_MAX_CONTEXT_TOKENS=800
export ONNX_MAX_TOKENS=100
export ONNX_SLIDING_WINDOW=true
```

### Scenario 4: Repetitive Output

**Problem:** Model repeats same phrases
**Solutions:**
1. Increase repetition penalty
2. Adjust temperature
3. Change top_p/top_k

```bash
export ONNX_REPETITION_PENALTY=1.2
export ONNX_TEMPERATURE=0.4
export ONNX_TOP_P=0.85
```

---

## Debug and Logging

### `DEBUG`
**Values:** `true` | `false`
**Default:** `false`
**Description:** Enable detailed logging

```bash
export DEBUG=true
npx agentic-flow --agent coder --task "test"
```

### `ONNX_LOG_PERFORMANCE`
**Values:** `true` | `false`
**Default:** `false`
**Description:** Log performance metrics

```bash
export ONNX_LOG_PERFORMANCE=true
# Outputs: tokens/sec, latency, context size, etc.
```

---

## Example Workflows

### Daily Development (Local, Free)

```bash
# .env file
PROVIDER=onnx
ONNX_OPTIMIZED=true
ONNX_TEMPERATURE=0.3
ONNX_MAX_TOKENS=200
ONNX_EXECUTION_PROVIDERS=cpu

# Usage
npx agentic-flow --agent coder --task "Build feature"
```

### CI/CD Pipeline (Fast, Local)

```bash
# CI environment variables
PROVIDER=onnx
ONNX_OPTIMIZED=true
ONNX_MAX_CONTEXT_TOKENS=800
ONNX_MAX_TOKENS=100
ONNX_TEMPERATURE=0.2
```

### Hybrid: ONNX + Cloud Fallback

```bash
# Try ONNX first (80% of tasks)
export PROVIDER=onnx
export ONNX_OPTIMIZED=true

# For complex tasks, switch to cloud
unset PROVIDER
export ANTHROPIC_API_KEY=sk-ant-...
# or
export OPENROUTER_API_KEY=sk-or-v1-...
```

---

## Best Practices

1. **Always enable optimizations**
   ```bash
   export ONNX_OPTIMIZED=true
   ```

2. **Lower temperature for code**
   ```bash
   export ONNX_TEMPERATURE=0.3
   ```

3. **Enable GPU if available** (30x faster!)
   ```bash
   export ONNX_EXECUTION_PROVIDERS=cuda,cpu
   ```

4. **Keep context under 2K tokens** (2-4x faster)
   ```bash
   export ONNX_MAX_CONTEXT_TOKENS=1500
   ```

5. **Use `.env` file** for consistency
   ```bash
   # Create .env file in project root
   echo "PROVIDER=onnx" >> .env
   echo "ONNX_OPTIMIZED=true" >> .env
   echo "ONNX_TEMPERATURE=0.3" >> .env
   ```

---

## Troubleshooting

### Error: Model not found
```bash
# Check model path
ls -lh ./models/phi-4-mini/cpu_and_mobile/cpu-int4-rtn-block-32-acc-level-4/

# Re-download if missing
rm -rf ./models/phi-4-mini
npx agentic-flow --agent coder --task "test" --provider onnx
```

### Error: CUDA not available
```bash
# Check CUDA installation
nvidia-smi

# Fall back to CPU
export ONNX_EXECUTION_PROVIDERS=cpu
```

### Slow inference (< 10 tok/s)
```bash
# Enable optimizations
export ONNX_OPTIMIZED=true
export ONNX_MAX_CONTEXT_TOKENS=1000
export ONNX_SLIDING_WINDOW=true

# Best: Enable GPU
export ONNX_EXECUTION_PROVIDERS=cuda,cpu
```

---

## See Also

- [ONNX CLI Usage Guide](./ONNX_CLI_USAGE.md)
- [ONNX Optimization Guide](./ONNX_OPTIMIZATION_GUIDE.md)
- [ONNX vs Claude Quality](./ONNX_VS_CLAUDE_QUALITY.md)
- [Full ONNX Integration](./ONNX_INTEGRATION.md)

---

**Remember:** ONNX is free and runs locally. Optimize first, then decide if you need cloud providers for complex tasks.
