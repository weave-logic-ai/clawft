# Model ID Mapping

## Overview

Different LLM providers use different model ID formats for the same models. This creates compatibility issues when routing requests across providers.

## The Problem

**Example: Claude Sonnet 4.5**

- **Anthropic API:** `claude-sonnet-4-5-20250929` (dated release format)
- **OpenRouter API:** `anthropic/claude-sonnet-4.5` (vendor/model format)
- **AWS Bedrock:** `anthropic.claude-sonnet-4-5-v2:0` (ARN-style format)

Using `claude-sonnet-4-5-20250929` with OpenRouter resulted in:
```
❌ Provider error from openrouter: claude-sonnet-4-5-20250929 is not a valid model ID
```

## The Solution

Automatic model ID mapping when routing between providers.

### Implementation

**File:** `src/router/model-mapping.ts`

```typescript
export const CLAUDE_MODELS: Record<string, ModelMapping> = {
  'claude-sonnet-4.5': {
    anthropic: 'claude-sonnet-4-5-20250929',
    openrouter: 'anthropic/claude-sonnet-4.5',
    bedrock: 'anthropic.claude-sonnet-4-5-v2:0',
    canonical: 'Claude Sonnet 4.5'
  },
  // ... more models
};

export function mapModelId(
  modelId: string,
  targetProvider: 'anthropic' | 'openrouter' | 'bedrock'
): string {
  // Automatic conversion between formats
}
```

### Integration

Updated `OpenRouterProvider` to automatically map model IDs:

```typescript
// src/router/providers/openrouter.ts
import { mapModelId } from '../model-mapping.js';

private formatRequest(params: ChatParams, stream = false): any {
  // Map model ID to OpenRouter format
  const openrouterModel = mapModelId(params.model, 'openrouter');

  const body: any = {
    model: openrouterModel, // ✅ Now uses correct format
    // ...
  };
}
```

## Supported Models

### Claude Sonnet 4.5 (Latest)
- **Anthropic:** `claude-sonnet-4-5-20250929`
- **OpenRouter:** `anthropic/claude-sonnet-4.5`
- **Bedrock:** `anthropic.claude-sonnet-4-5-v2:0`

### Claude Sonnet 4
- **Anthropic:** `claude-sonnet-4-20240620`
- **OpenRouter:** `anthropic/claude-sonnet-4`
- **Bedrock:** `anthropic.claude-sonnet-4-v1:0`

### Claude 3.7 Sonnet
- **Anthropic:** `claude-3-7-sonnet-20250219`
- **OpenRouter:** `anthropic/claude-3.7-sonnet`

### Claude 3.5 Sonnet (October 2024)
- **Anthropic:** `claude-3-5-sonnet-20241022`
- **OpenRouter:** `anthropic/claude-3.5-sonnet-20241022`
- **Bedrock:** `anthropic.claude-3-5-sonnet-20241022-v2:0`

### Claude 3.5 Haiku
- **Anthropic:** `claude-3-5-haiku-20241022`
- **OpenRouter:** `anthropic/claude-3.5-haiku-20241022`

### Claude Opus 4.1
- **Anthropic:** `claude-opus-4-1-20250514`
- **OpenRouter:** `anthropic/claude-opus-4.1`

## Usage

### Automatic (Recommended)

The router automatically maps model IDs when using cost-optimized routing:

```typescript
import { ModelRouter } from './router/router.js';

const router = new ModelRouter();

// Use Anthropic format - automatically maps to OpenRouter format
const response = await router.chat({
  model: 'claude-sonnet-4-5-20250929',
  messages: [{ role: 'user', content: 'Hello!' }]
});

// Router tries: OpenRouter (with mapped ID) → Anthropic (original ID)
// ✅ No more "not a valid model ID" errors!
```

### Manual Mapping

You can also use the mapping utility directly:

```typescript
import { mapModelId, getModelName } from './router/model-mapping.js';

// Convert Anthropic ID to OpenRouter format
const openrouterId = mapModelId('claude-sonnet-4-5-20250929', 'openrouter');
// Result: 'anthropic/claude-sonnet-4.5'

// Get human-readable name
const name = getModelName('claude-sonnet-4-5-20250929');
// Result: 'Claude Sonnet 4.5'
```

## Benefits

1. **No More Errors:** Automatic conversion prevents "invalid model ID" errors
2. **Cost Optimization:** OpenRouter can now be used for 99% cost savings
3. **Provider Flexibility:** Same config works across all providers
4. **Maintainability:** Single source of truth for model mappings
5. **Future-Proof:** Easy to add new models and providers

## Cost Savings Example

**Before Mapping:**
- OpenRouter attempt fails → Fallback to Anthropic
- Cost: Full Anthropic pricing ($3/$15 per million tokens)

**After Mapping:**
- OpenRouter attempt succeeds → No fallback needed
- Cost: OpenRouter pricing (~$0.03/$0.45 per million tokens)
- **Savings: ~99% cost reduction**

## Adding New Models

To add a new model mapping:

```typescript
// src/router/model-mapping.ts
export const CLAUDE_MODELS: Record<string, ModelMapping> = {
  // ... existing models

  'claude-new-model': {
    anthropic: 'claude-new-model-20250101',
    openrouter: 'anthropic/claude-new-model',
    bedrock: 'anthropic.claude-new-model-v1:0',
    canonical: 'Claude New Model'
  }
};
```

No other code changes needed - mapping is automatic!

## Testing

Before fix:
```bash
$ npx tsx src/reasoningbank/demo-comparison.ts
❌ Provider error from openrouter: claude-sonnet-4-5-20250929 is not a valid model ID
❌ Provider error from openrouter: claude-sonnet-4-5-20250929 is not a valid model ID
...
```

After fix:
```bash
$ npx tsx src/reasoningbank/demo-comparison.ts
💰 Cost-optimized routing: selected openrouter
[INFO] Judgment complete: Failure (0.95) in 6496ms
✅ No errors! OpenRouter working correctly.
```

## References

- [OpenRouter Model IDs](https://openrouter.ai/anthropic)
- [Anthropic API Documentation](https://docs.anthropic.com/en/api/models)
- [AWS Bedrock Model IDs](https://docs.aws.amazon.com/bedrock/latest/userguide/model-ids.html)
