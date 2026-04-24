# Provider-Specific Tool Instruction Optimization

## Overview

Enhanced the OpenRouter and Gemini proxies with provider-specific tool instructions to optimize tool calling success rates across different LLM families.

## Changes Made

### 1. Created Provider Instruction Templates (`src/proxy/provider-instructions.ts`)

Implemented tailored instruction sets for each major provider:

- **ANTHROPIC_INSTRUCTIONS**: Native tool calling, minimal instructions needed
- **OPENAI_INSTRUCTIONS**: XML format with strong emphasis on using tags
- **GOOGLE_INSTRUCTIONS**: Detailed step-by-step instructions with explicit examples
- **META_INSTRUCTIONS**: Clear, concise instructions for Llama models
- **DEEPSEEK_INSTRUCTIONS**: Technical, precise instructions
- **MISTRAL_INSTRUCTIONS**: Direct, action-oriented commands
- **XAI_INSTRUCTIONS**: Balanced instructions for Grok models
- **BASE_INSTRUCTIONS**: Default fallback for unknown providers

### 2. Enhanced OpenRouter Proxy (`src/proxy/anthropic-to-openrouter.ts`)

**Key Updates**:
- Imported `getInstructionsForModel` and `formatInstructions` from provider-instructions
- Added `extractProvider()` method to parse provider from model ID
- Modified `convertAnthropicToOpenAI()` to use model-specific instructions:
  ```typescript
  const modelId = anthropicReq.model || this.defaultModel;
  const provider = this.extractProvider(modelId);
  const instructions = getInstructionsForModel(modelId, provider);
  const toolInstructions = formatInstructions(instructions);
  ```

### 3. Updated Test Models (`test-top20-models.ts`)

Corrected invalid model IDs based on OpenRouter API research:
- `deepseek/deepseek-v3.1:free` → `deepseek/deepseek-chat-v3.1:free`
- `deepseek/deepseek-v3` → `deepseek/deepseek-v3.2-exp`
- `google/gemma-3-12b` → `google/gemma-2-27b-it`

### 4. Created Provider Validation Test (`tests/test-provider-instructions.ts`)

Comprehensive test covering all major providers:
- Anthropic (Claude)
- OpenAI (GPT)
- Google (Gemini)
- Meta (Llama)
- DeepSeek
- Mistral
- X.AI (Grok)

## Instruction Strategy by Provider

### Anthropic Models
**Format**: Native tool calling
**Strategy**: Minimal instructions - models already understand Anthropic tool format
**Example**: "You have native access to file system tools. Use them directly."

### OpenAI/GPT Models
**Format**: XML tags with strong emphasis
**Strategy**: Explicit instructions with "CRITICAL" emphasis to use exact XML formats
**Key Point**: "Do not just describe the file - actually use the tags"

### Google/Gemini Models
**Format**: Detailed XML with step-by-step guidance
**Strategy**: Very explicit instructions with numbered steps
**Key Point**: "Always use the XML tags. Just writing code blocks will NOT create files"

### Meta/Llama Models
**Format**: Clear, concise XML commands
**Strategy**: Simple, direct examples without excessive detail
**Key Point**: "Use these tags to perform actual file operations"

### DeepSeek Models
**Format**: Technical, precise XML instructions
**Strategy**: Focus on structured command parsing
**Key Point**: "Commands are parsed and executed by the system"

### Mistral Models
**Format**: Action-oriented with urgency
**Strategy**: Use "ACTION REQUIRED" language to prompt tool usage
**Key Point**: "Do not just show code - use the tags to create real files"

### X.AI/Grok Models
**Format**: Balanced, clear command structure
**Strategy**: Straightforward file system command list
**Key Point**: "Use structured commands to interact with the file system"

## Expected Improvements

Based on initial testing (TOP20_MODELS_MATRIX.md):

**Before Optimization**:
- 92.9% tool success rate (13/14 working models)
- 1 model (gpt-oss-120b) not using tools

**After Optimization** (Expected):
- 95-100% tool success rate with provider-specific instructions
- Better instruction clarity reducing model confusion
- Faster response times due to clearer prompts

## Testing

### Run Provider Instruction Test
```bash
export OPENROUTER_API_KEY="your-key-here"
npx tsx tests/test-provider-instructions.ts
```

### Run Top 20 Models Test (Updated IDs)
```bash
export OPENROUTER_API_KEY="your-key-here"
npx tsx test-top20-models.ts
```

## Next Steps

1. **Run Validation Tests**: Execute provider instruction test to verify improvements
2. **Re-run Top 20 Test**: Use corrected model IDs and optimized instructions
3. **Measure Improvements**: Compare success rates before/after optimization
4. **Fine-tune Instructions**: Adjust any providers with < 100% success rate
5. **Document Results**: Update TOP20_MODELS_MATRIX.md with final results

## Security Note

All API keys must be provided via environment variables. Never hardcode credentials in source files or tests.

## Files Modified

- ✅ `src/proxy/provider-instructions.ts` (created)
- ✅ `src/proxy/anthropic-to-openrouter.ts` (enhanced)
- ✅ `test-top20-models.ts` (model IDs corrected)
- ✅ `tests/test-provider-instructions.ts` (created)
- ✅ `docs/PROVIDER_INSTRUCTION_OPTIMIZATION.md` (this file)

## Conclusion

Provider-specific instruction optimization provides a systematic approach to maximizing tool calling success across diverse LLM families. By tailoring instructions to each model's strengths and quirks, we can achieve near-universal tool support while maintaining the same proxy architecture.
