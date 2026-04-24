# OpenRouter Proxy Issues and Required Fixes

**Status:** 🔴 CRITICAL - v1.1.13 release claims do not match actual behavior

**Date:** 2025-10-05

---

## Summary

The v1.1.13 release claimed "100% success rate" for OpenRouter providers, but actual testing shows all three providers (GPT-4o-mini, DeepSeek, Llama 3.3) have critical issues:

1. **Models inappropriately trying to use tools** for simple code generation
2. **Truncated responses** ending with `<function=` or similar
3. **Malformed tool call syntax** in model outputs
4. **Context detection not working** properly

---

## Root Causes Identified

### 1. Context Detection Broken

**File:** `src/proxy/provider-instructions.ts:215-226`

**Problem:**
```typescript
const fileKeywords = [
  'create file', 'write file', 'save to', 'create a file',
  // ...
];
```

Task: "Create a Python file at /tmp/test.py" → Returns `false` (should be `true`)

**Reason:** Keyword matching is too strict. "Create a Python file" doesn't match "create file" or "create a file".

**Fix Required:**
```typescript
const fileKeywords = [
  /create.*file/i,
  /write.*file/i,
  /save.*to/i,
  /save.*file/i,
  /write.*to disk/i,
  /create.*script/i,
  /make.*file/i
];

return fileKeywords.some(pattern => pattern.test(combined));
```

### 2. XML Instructions Still Being Injected

**File:** `src/proxy/anthropic-to-openrouter.ts:204-211`

**Problem:** Even when `taskRequiresFileOps()` returns `false`, models are still receiving tool instructions somehow.

**Debug needed:**
- Add logging to see what instructions are actually being sent
- Verify `formatInstructions()` is being called with correct `includeXmlInstructions` parameter

### 3. Models Returning Malformed Tool Calls

**Observed Output:**
```
[Executing: python -c "..."]<function=
```

**Problem:** Models are trying to use tools incorrectly:
- Mixing text output with tool calls
- Incomplete tool call syntax
- Wrong tool call format

**Possible Causes:**
1. Models confused by XML instruction format
2. Max tokens too low (response truncated mid-tool-call)
3. Models not understanding when to/not to use tools

**Fix Required:**
- REMOVE XML instructions entirely for OpenRouter models
- Use ONLY native OpenAI function calling format
- Let OpenRouter handle tool calling natively

### 4. parseStructuredCommands() Inappropriate

**File:** `src/proxy/anthropic-to-openrouter.ts:286-338`

**Problem:** This function tries to parse XML tags from model output, but:
1. Models aren't reliably producing valid XML
2. Models are mixing text with XML
3. Truncated responses create malformed XML

**Fix Required:**
OpenRouter models should use native OpenAI tool calling:
```typescript
// DON'T parse XML from text
// DO use message.tool_calls from OpenAI response format

const tool_calls = message.tool_calls || [];
// These are already in correct format from OpenRouter
```

---

## Proposed Solution

### Phase 1: Stop Injecting XML Instructions for OpenRouter

```typescript
// In convertAnthropicToOpenAI():

// NEVER inject XML instructions for OpenRouter
const toolInstructions = config.provider === 'openrouter'
  ? 'Respond with clean, well-formatted code.'
  : formatInstructions(instructions, needsFileOps);
```

### Phase 2: Use Native OpenAI Tool Calling

```typescript
// OpenRouter models should use tools ONLY via OpenAI function calling
// NOT via XML tags in text

if (anthropicReq.tools && anthropicReq.tools.length > 0) {
  // Convert MCP tools to OpenAI format (already done)
  openaiReq.tools = anthropicReq.tools.map(tool => ({
    type: 'function',
    function: {
      name: tool.name,
      description: tool.description,
      parameters: tool.input_schema
    }
  }));
}
```

### Phase 3: Handle Tool Calls Correctly in Response

```typescript
// In convertOpenAIToAnthropic():

// ONLY look at message.tool_calls from OpenAI
// DON'T try to parse XML from message.content

const toolCalls = message.tool_calls || [];

if (toolCalls.length > 0) {
  // Model wants to use tools - convert to Anthropic format
  contentBlocks = toolCalls.map(tc => ({
    type: 'tool_use',
    id: tc.id,
    name: tc.function.name,
    input: JSON.parse(tc.function.arguments)
  }));
} else {
  // Pure text response - no tool use
  contentBlocks = [{
    type: 'text',
    text: message.content
  }];
}
```

---

## Testing Required

### Test Matrix

| Test Case | Provider | Model | Task | Expected |
|-----------|----------|-------|------|----------|
| 1 | openrouter | gpt-4o-mini | "Write Python function to add numbers. Just show code." | Clean Python code, NO tool calls |
| 2 | openrouter | gpt-4o-mini | "Create file /tmp/test.py with add function" | Use Write tool, create file |
| 3 | openrouter | deepseek-chat | "Write multiply function. Just code." | Complete code, no truncation |
| 4 | openrouter | deepseek-chat | "Save multiply function to /tmp/mult.py" | Use Write tool |
| 5 | openrouter | llama-3.3-70b | "Write subtract function. Show code." | Code without prompt repetition |
| 6 | openrouter | llama-3.3-70b | "Create /tmp/sub.py with subtract" | Use Write tool |

### Validation Command

```bash
npm run validate:openrouter
```

**Must pass ALL 6 tests** before claiming fixes work.

---

## Immediate Actions

1. **[ ] Fix taskRequiresFileOps()** - Use regex patterns instead of exact string matching
2. **[ ] Remove XML instructions for OpenRouter** - Never inject XML for OR models
3. **[ ] Fix convertOpenAIToAnthropic()** - Don't parse XML, use tool_calls only
4. **[ ] Add comprehensive logging** - See what's actually being sent/received
5. **[ ] Run full test matrix** - Validate ALL cases before release
6. **[ ] Update VALIDATION-RESULTS.md** - With REAL test results
7. **[ ] Update CHANGELOG** - Acknowledge issues, document fixes

---

## Package Issues to Fix

1. **Missing validation scripts in npm package**
   - Add `validation/` to package.json files array ✅ (done)
   - Add `scripts/` to package.json files array ✅ (done)

2. **Broken validate:openrouter script**
   - Points to non-existent file
   - Needs to point to working validation script

3. **Documentation inconsistency**
   - Release notes claim 100% success
   - Actual behavior: 0% success for OpenRouter
   - Need honest documentation

---

## Timeline

**URGENT:** These are critical bugs affecting core functionality

- **Immediate (today):** Implement fixes 1-3
- **Before next release:** Complete testing and validation
- **Update release notes:** Be honest about what works and what doesn't

---

## Success Criteria

✅ **Definition of Done:**

1. All 6 test cases in matrix pass
2. `npm run validate:openrouter` passes with 100% success
3. Real-world usage confirmed (not just unit tests)
4. Documentation accurately reflects capabilities
5. No false claims in release notes

---

## Current Status by Provider

| Provider | Code Gen | File Ops | Tool Calling | Status |
|----------|----------|----------|--------------|--------|
| Anthropic | ✅ Perfect | ✅ Perfect | ✅ Perfect | ✅ Production Ready |
| Gemini | ✅ Perfect | ✅ Perfect | ✅ Perfect | ✅ Production Ready |
| OpenRouter GPT-4o-mini | ❌ Tool calls inappropriately | ❌ Malformed | ❌ Truncated | 🔴 BROKEN |
| OpenRouter DeepSeek | ❌ Malformed output | ❌ Wrong format | ❌ Incomplete | 🔴 BROKEN |
| OpenRouter Llama 3.3 | ❌ Truncated | ❌ Fails | ❌ Broken | 🔴 BROKEN |

---

## Recommended User Communication

**Honesty is the best policy:**

```markdown
## v1.1.13 Status Update

**Working Providers:**
- ✅ Anthropic (direct) - Fully tested, production ready
- ✅ Google Gemini - Fully tested, FREE tier available

**Known Issues:**
- ⚠️ OpenRouter proxy has tool calling format issues
- ⚠️ Working on fixes, will release v1.1.14 when validated
- ⚠️ Use Anthropic or Gemini for production until fixed

**If you need OpenRouter:**
- Use agentic-flow CLI directly (works)
- Don't use proxy mode until v1.1.14
```

---

**Author:** Validation testing
**Last Updated:** 2025-10-05
